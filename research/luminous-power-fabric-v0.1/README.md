# Luminous Power Fabric v0.1

**Issue:** [#364 — Luminous Power Fabric](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Status:** research specification / pre-hardware design  
**Evidence tier:** literature-backed design proposal; no circuit has been built or physically measured  
**Snapshot:** 2026-11-10

## Purpose

Luminous Power Fabric is a solid-state-first, modular and repairable electrical power platform.
It is intended to span low-voltage DC building blocks, microgrids, commercial distribution
equipment and (only after specialist qualification) utility-scale equipment.

The programme is not a proposal to replace every conventional transformer, breaker or AC
distribution system with semiconductor devices. It seeks the best system architecture for each
application. Hybrid mechanical/electronic equipment remains a first-class candidate where it
reduces conduction loss, cost, thermal stress or failure risk.

The central design unit is a **Power Electronics Building Block (PEBB)** with versioned electrical,
mechanical, thermal, control, protection, service and evidence interfaces. Product families should
compose qualified blocks instead of re-inventing every power stage.

## Paradigm shifts

1. **From one-off devices to composable power blocks.** Standardize ports, control boundaries,
   thermal/mechanical envelopes, telemetry, diagnostics, service replacement and evidence.
2. **From AC-versus-DC ideology to conversion-chain accounting.** Keep AC compatibility; add
   DC-native islands where measured end-to-end efficiency, material, resilience or cost improves.
3. **From AI-controlled power to intelligence-assisted engineering.** Symthaea may propose and
   optimize; local deterministic controllers regulate; independent hardware protection constrains
   hazardous states. AI, remote services and communications are never required for safe shutdown.
4. **From rated-life marketing to mission-profile reliability.** Design around real load, ambient,
   thermal cycling, humidity, switching, grid events and maintenance profiles.
5. **From replacement-first to repair-first product architecture.** Replace failed power modules,
   fans, capacitors, sensors and control boards without discarding sound assemblies, subject to
   requalification of affected claims.
6. **From product telemetry to evidence-bearing equipment.** Component identity, revision, BOM,
   firmware, calibration, test conditions, repair and end-of-life records are linked to each unit.
7. **From peak efficiency to lifetime energy value.** Optimize conversion losses, standby energy,
   cooling, service time, replacement cost, outage exposure and recycling together.
8. **From simulation confidence to staged physical qualification.** Simulation informs design; it
   does not substitute for measurement, protection tests, accredited conformity assessment or
   field evidence.

## First demonstrator: PEBB-LV-01

A candidate **isolated, bidirectional, 48 V-class DC/DC module**:

| Parameter | v0.1 design envelope |
|---|---|
| Nominal port voltage | 48 V DC |
| Candidate operating range | 36–60 V DC on each port |
| Continuous output-power target | 500 W |
| Direction | Bidirectional |
| Galvanic isolation | Required; rating and test levels remain design inputs |
| Cooling | Compare passive and forced-air designs |
| Operating mode | Current-limited laboratory source/load only |
| Grid/mains connection | Explicitly out of scope |
| Battery packs | Explicitly out of scope for phase 0 |

The 36–60 V range is a design envelope, not a safety classification. Low voltage does not eliminate
arc, fire, burn, stored-energy or high-current hazards. The first build requires current-limited
sources, correctly rated DC protection, guarded terminals, safe discharge, documented isolation,
a reviewed hazard analysis and competent supervision.

The topology is not frozen. Compare:
- Dual-active-bridge (DAB), including phase-shift and modulation alternatives.
- Bidirectional CLLC / resonant isolated conversion.
- A four-switch non-isolated buck-boost as an efficiency, cost and complexity reference only.

The non-isolated reference is not a substitute for the required galvanic isolation. The topology
decision must include full-range efficiency, reverse-flow performance, circulating current,
transformer and capacitor stress, control complexity, thermal behavior, EMI risk, manufacturability,
repairability and component availability.

## Product-family map

| Product family | Reusable blocks | Programme stage |
|---|---|---|
| Low-voltage DC/DC | Isolated and non-isolated PEBBs, sensing, protection | First |
| Home / commercial energy hub | DC/DC, AC/DC, storage and load interfaces | Later |
| Microgrid controller and interface | Grid-forming inverter, source/load coordination, protection | Later |
| DC distribution and protection | Solid-state and hybrid breakers, sectionalizers, busbars | Later |
| Commercial grid conversion | Modular AC/DC, active filters, STATCOM interfaces | Later |
| Solid-state transformer | MV front end, modular DC/DC PEBBs, medium-frequency transformer | Research frontier; specialist partnership |

## Luminous system composition

| System | Responsibility | Explicit boundary |
|---|---|---|
| Symthaea | Literature synthesis, design-space search, trade-off reasoning, hypothesis generation | Cannot certify a design, override protection or directly command hazardous hardware |
| Symtropy | Converter/system models, coupled electrical-thermal-mechanical simulation, fault replay, HIL scenario design | Simulation is not physical evidence; model validation is separately recorded |
| Sol-Atlas | 3D geometry, packaging, clearances, thermal interfaces, wiring, BOM/revision and manufacturing digital thread | Geometry does not prove dielectric, thermal or mechanical qualification |
| Mycelix | Component/lot provenance, firmware and calibration lineage, test evidence, repair history and independent verification | A ledger or signature does not prove physical truth, supplier quality or independence |
| Deterministic embedded control | Local regulation, bounded operating modes and state transitions | No cloud, model provider or network dependency in the fast control loop |
| Independent protection | Hardware current/voltage/temperature constraints, fusing, isolation and safe de-energization | Must remain effective if the main controller fails |

Reuse existing Luminous inventory, evidence, authorization and lifecycle systems. Do not build another
device identity, fleet manager, update service, evidence ledger or general-purpose control plane.

## Evidence and claim vocabulary

Every claim is labelled with its highest actually attained tier:

1. **Conceptual** — hypothesis or product idea.
2. **Specified** — requirements, interfaces and acceptance criteria defined.
3. **Modelled** — model executes; assumptions and model validation state disclosed.
4. **Structurally checked** — schemas, invariants or design rules checked.
5. **CI-tested** — exact source revision and test outputs retained.
6. **Bench-measured** — physical prototype measured with instrument and calibration records.
7. **Independently reproduced** — another qualified party reproduces the result.
8. **Conformity assessed / certified** — applicable accredited or authorized process completed.
9. **Field validated** — defined installation, population, duration and operating conditions evidenced.

A design specification, synthetic fixture, simulation, skipped CI job or queued runner cannot be
reported as a physical pass, certified product, reliability guarantee or better-than-SOTA result.

## Work packages

- WP0 — requirements, hazard analysis, standards applicability and measurement contract.
- WP1 — topology and semiconductor down-selection.
- WP2 — power-stage, magnetics, gate-drive, sensing, thermal and mechanical design.
- WP3 — model correlation and offline measurement-evidence tooling.
- WP4 — current-limited low-voltage demonstrator and fault-injection programme.
- WP5 — mission-profile reliability, repairability and supplier qualification.
- WP6 — microgrid AC/DC interface and controller-HIL research.
- WP7 — accredited pre-compliance, certification and productization with specialist partners.

## Gate 0 exit criteria

Before a schematic is frozen: requirements trace to tests; DAB/CLLC/reference comparison is complete;
the electrical safety and insulation approach is reviewed by a qualified power-electronics engineer;
protection and safe-state boundaries are documented; candidate components have datasheets and
lifecycle/supplier status; measurement uncertainty is planned; and the phase-0 no-grid/no-mains
boundary is reflected in the lab procedure.

## Further topology research

See [PARADIGM_SHIFT_RESEARCH.md](./PARADIGM_SHIFT_RESEARCH.md) for current literature on reconfigurable
DAB–CLLC building blocks, low-voltage GaN-versus-silicon trade-offs, CLLC control strategies and
repair-first modular power architecture. It updates the research direction without freezing a
topology for PEBB-LV-01.

## Industry comparison

See [INDUSTRY_BASELINE.md](./INDUSTRY_BASELINE.md) for current 500 W commercial comparators and
the matched head-to-head plan. In particular, 97% efficiency is not by itself a SOTA claim.

## Loss engineering and topology research

The next modelling increment is documented in [LOSS_BUDGET_AND_MODEL_PLAN.md](./LOSS_BUDGET_AND_MODEL_PLAN.md).
It introduces a provenance-aware Rust loss-accounting engine, an ideal DAB SPS power-transfer /
phase-shift / inductance screen, and specifies the inputs required before non-ideal DAB/CLLC topology
losses can be calculated. The June 2026 500 W CLLC paper's 97.6% peak forward result is now included
as a same-power literature anchor, but not yet a matched benchmark.

## New CLLC research: gain plus internal-state feasibility

The October 2026 research adds a state-constrained T-type impedance-network method for bidirectional
CLLC design. Our Rust library now implements the first, limited layer: symmetric FHA gain evaluation
and a discrete sampled gain-curve scan. It does **not** yet implement the T-network, impedance angle,
internal-state constraints, Coss/dead-time commutation or ZVS feasibility. See
[PARADIGM_SHIFT_RESEARCH.md](./PARADIGM_SHIFT_RESEARCH.md) and
[LOSS_BUDGET_AND_MODEL_PLAN.md](./LOSS_BUDGET_AND_MODEL_PLAN.md).

## CLLC FHA screening implementation

The Rust crate also evaluates the symmetric CLLC first-harmonic-approximation gain equation and
scans discrete normalized-frequency points. Charging and discharging use direction-specific
equivalent quality factors. This is the first layer only; the October 2026 T-type impedance-network
method motivates the next internal-state, impedance-angle and ZVS feasibility layer.

## CLLC internal-state modelling increment

Beyond the FHA gain screen, the Rust tooling now includes a generic complex T-network phasor
evaluator. It calculates Y parameters, secondary/primary voltage phasor ratio and primary input
impedance from caller-supplied signed branch reactances and secondary complex load impedance.
Mapping actual CLLC components and secondary modulation into those inputs, then checking paper-state
constraints and ZVS, remains engineering work. See the model plan for the exact claim boundary.

The T-network module also screens the published low-frequency boost-mode frequency interval
and intersects it with controller limits. Other CLLC state, ZVS and maximum-gain constraints remain
separate unimplemented work.

The CLLC T-network layer now maps physical resonant L/C/magnetizing parameters into signed
reactances, evaluates the paper-specific VF-TPS bridge impedance and classifies all sixteen
P/N sign states. It can identify the selected PNNN low-frequency boost state; PNNN is not by itself
a ZVS or safety qualification.

## Updated competitive strategy

A 7.3 kW CLLLC-RDAB paper now provides a full-profile 97.1–97.8% efficiency reference with nearly
symmetric reverse efficiency. A 1 kW DAB paper reinforces multi-objective design across ZVS range,
rating utilization and reactive power. Therefore the programme will target mission-weighted
bidirectional efficiency and full-envelope feasibility—not a single peak number. See
[INDUSTRY_BASELINE.md](./INDUSTRY_BASELINE.md) and
[BENCHMARKS.md](./BENCHMARKS.md).

## DAB physics implementation

The Rust tools now include the literature-derived lossless SPS piecewise power/RMS-current model,
FHA-error and analytic ZVS-polarity screens, a parameterized one-node thermal estimate, and a
separate periodic resistive RL solver with energy-balance closure. The research boundary and
equations are documented in [DAB_SPS_PHYSICS_MODEL.md](./DAB_SPS_PHYSICS_MODEL.md).

## CLLC PNNN operating-state screen

The physical tank mapper, VF-TPS bridge equivalent and T-network now compose into a PNNN low-frequency
boost-state screen. It rejects a tank that violates the paper's equal-resonant-frequency / equal-`k`
assumptions or falls outside the controller-intersected switching-frequency window. This is still
only one design constraint, not a complete ZVS or gain qualification.

## DAB mission-profile design-space sweep

The Rust engineering tools now evaluate frozen, weighted DAB mission points with direction-correct
power accounting, resistive periodic current, analytic ZVS-polarity and thermal screens, and a
multiobjective Pareto filter. Candidate comparisons retain every infeasible point rather than
silently pruning it. Results are only model-based screening, not measured performance or a design
approval. See [DAB_SPS_PHYSICS_MODEL.md](./DAB_SPS_PHYSICS_MODEL.md).

## Multi-family product architecture

The programme now separates low-voltage PEBBs, 48 V power conversion, 400–800 VDC facility/rack
conversion, microgrid interfaces, DC protection and later SST systems. Shared contracts and evidence
do not imply common hardware or identical qualification. See
[PRODUCT_FAMILY_ARCHITECTURE.md](./PRODUCT_FAMILY_ARCHITECTURE.md).

The Rust research tooling now also evaluates the October 2026 paper's analytical CLLC parameter
constraints—symmetry, constant-power Q, unity-gain bound, boost-frequency window and maximum-gain
boundary. This rejects some poor parameter candidates before switching simulation; it does not
establish actual ZVS. See [LOSS_BUDGET_AND_MODEL_PLAN.md](./LOSS_BUDGET_AND_MODEL_PLAN.md).

## PF-HVDC-6K: separate 800 VDC to 50 V research track

The 800 VDC data-centre opportunity now has a dedicated research plan:
[PF-HVDC-6KW_RESEARCH_PLAN.md](./PF-HVDC-6KW_RESEARCH_PLAN.md). It is deliberately separate from
the 500 W bidirectional PEBB-LV-01. Current research comparators include a 6 kW 800 V/50 V GaN LLC
prototype reporting 98.75% peak and 2,070 W/in³, and a separate ISOP LLC paper reporting 98.1%
full-load and >98.4% peak. These are different unidirectional research designs; neither is a
Luminous result or a matched bidirectional benchmark.

## Whole-chain energy accounting implementation

A read-only Rust conversion-chain auditor now evaluates end-to-end input energy, delivered energy,
conversion losses, standby draw, annual electricity cost and grid emissions across a complete
8,760-hour piecewise load profile. A matched comparison helper rejects scenarios with different
delivered energy, hours, tariff/currency or grid-carbon intensity before calculating energy, cost and
emissions savings. It keeps source provenance for each stage efficiency, standby, tariff and
emissions factor. This enables fairer AC/DC, 48 V and 800 V architecture comparisons than adding
separate peak-efficiency percentages. It does not replace a site model or measured energy study.

## Current limitations

This package contains analytical screening models and offline data-audit tools; it does **not**
contain a validated non-ideal switching model, a fabricated PCB, a power-stage prototype, calibrated
physical measurements, HIL correlation, production tooling or safety certification. The DAB and
CLLC models remain bounded by their documented ideal/FHA/equation assumptions. No result is a
physical ZVS guarantee or permission to energize hardware. High-voltage and grid-connected product
families remain architecture/research tracks only. Hardware energization remains gated on qualified
engineering review, fault/energy analysis, suitable laboratory containment and a reviewed procedure.

## Repeated-event grid-interactive compute qualification

The new [Grid-Interactive Compute Resource Qualification v0.1](./GRID_INTERACTIVE_COMPUTE_RESOURCE_QUALIFICATION_2026-11-11.md) adds an offline event-level evaluator for compute-derived demand response. It binds the claim to one resource, grid zone, point of interconnection, operating envelope, workload contract, frozen counterfactual and control action.

The evaluator subtracts independently measured non-compute load reductions and conservative uncertainty bounds, then evaluates service quality, useful-work retention, response, sustain, timestamped recovery and rebound across repeated events. The report retains the frozen contract, baseline/event evidence references and event/recovery intervals; it reports P10 credited compute capacity and a conditional one-sided Wilson lower confidence bound (z=1.2815515655 for 90%). It explicitly does not provide market accreditation or grid-service authorization.

This is a measurement protocol and synthetic evaluator only. Nineteen new tests are authored but not executed; there is no compiled, CI-tested, site-measured or field-validated result.

## Deterministic workload replay and counterfactual flexibility

The [Deterministic Workload Replay and Counterfactual Flexibility Benchmark v0.1](./DETERMINISTIC_WORKLOAD_REPLAY_AND_COUNTERFACTUAL_BENCHMARK_2026-11-11.md) adds an offline replay engine that evaluates the same normalized CPU/GPU/memory-constrained jobs under a full-capacity baseline and a dispatch-constrained candidate.

The deterministic scheduler uses ordered release, ready and completion sets; the strict-priority policy protects non-deferrable jobs, applies explicit latest-start and hard completion deadlines, and never preempts in-flight work. Both baseline and dispatch schedules now emit exact half-open CPU/GPU/memory occupancy segments plus timestamped useful-work completion events, bound to a replay clock/origin and schedule-evidence reference. Segment resource-time and completion work are conservation-checked. A typed admission restriction vector keeps CPU-millicore-ns, GPU-unit-ns and memory-MiB-ns separate. The event stream is intended to feed #382's PCC/useful-work audit; resource-time is not converted into watts or energy.

Twenty synthetic Rust tests are authored, including a 10,000-job fixture; they are **not executed**. No trace binaries were imported and no million-job throughput, power reduction, carbon reduction or grid-service claim is made.

## Calibrated workload-state power model

[Issue #401](https://github.com/Luminous-Dynamics/luminous-platform/issues/401) now has a read-only Rust state-power model. It fits configuration-specific empirical power distributions for workload states and ordered one-step transitions, using whole-session train/holdout separation. It requires exact hardware-configuration, measurement-boundary, workload and state-mapper matches; non-GOOD or irregular samples and unsupported state transitions fail closed. Prediction retains the supplied state chronology and carries calibration uncertainty into p10–p90 intervals.

The implementation does not infer prefill/decode/idle from abstract occupancy, does not synthesize a facility trace, and does not replace measured PCC energy or qualification. Its benchmark and SOTA comparison are specified in [CALIBRATED_WORKLOAD_STATE_POWER_MODEL_BENCHMARK_2026-11-11.md](./CALIBRATED_WORKLOAD_STATE_POWER_MODEL_BENCHMARK_2026-11-11.md). Fifteen synthetic Rust tests are authored but not executed; no physical or licensed trace data and no compile/runtime result are claimed.
 The model rejects training-run prediction targets, requires exact calibration/prediction clock-alignment method identity, and carries the source trace's state-mapping evidence reference into each prediction trace. It now separates held-out validation (`predict_power_trace`) from new operational scenario forecasts (`predict_operational_scenario`); scenario runs cannot reuse training/holdout IDs and cannot be scored as held-out validation.

## Regime-conditioned forecast calibration and monitoring

A new Rust layer follows #401's state-conditioned power model with regime-specific session-level split-conformal interval calibration. It treats a complete acquisition session as the statistical unit, preserves temporal dependence within that session, binds the entire upstream PowerModelContract and refuses calibrated outputs for unknown operating regimes.

The monitor reports whole-session coverage and a Wilson lower bound by regime, with explicit NOMINAL, DEGRADED, INSUFFICIENT_SUPPORT, SAMPLING_DESIGN_UNQUALIFIED and OUT_OF_DOMAIN statuses. Wilson monitoring bounds are withheld without a separate sampling-design review. It does not silently adapt the frozen calibrator; a new calibration requires a new revision and disjoint sessions.

This work is motivated by Yao et al.'s 6 November 2026 grid-oriented 448-GPU forecasting benchmark, which reports undercoverage after utilization shifts and shows that independent sampling of marginal forecasts produces unrealistic ramps and reserve costs. The Luminous reproduction plan also records the MIT Supercloud dataset's non-commercial/no-derivatives license and does not import it into commercial product work without rights review.

See [Regime-Conditioned Session Conformal Forecast Calibration](./REGIME_CONDITIONED_SESSION_CONFORMAL_FORECAST_CALIBRATION_2026-11-11.md). Twenty-three Rust tests are authored but not executed; no performance or physical qualification is claimed.


## Dependence-preserving temporal scenarios and horizon-wide reserve sizing

The temporal_scenarios Rust module adds independent marginal-quantile sampling as a negative control, complete residual-trajectory bootstrap with replacement, and Schaake-shuffle rank transfer. It binds exact forecast/model/regime/horizon inputs and preserves historical residual sample timestamps and evidence. Historical residual target windows must be complete, prior to the forecast origin and separated by at least one target horizon.

Scenario scoring includes multivariate energy and variogram scores, peak/ramp/energy distributions and PIT diagnostics. Horizon-wide reserve is derived from the critical fractile of maximum positive deviation from the scheduled median path, with the measured horizon retained for realized shortfall/cost evaluation. These are offline modelled scenario comparisons, not grid capacity or physical reserve credit.

The direct comparator is Yao et al.'s 6 November 2026 448-GPU GPU-IT forecast study. The MIT Supercloud source dataset has a CC BY-NC-ND 4.0 rights gate; no data are imported or used commercially by this implementation. See TEMPORAL_SCENARIOS_AND_HORIZON_RESERVE_BENCHMARK_2026-11-11.md.

Thirty-six Rust tests are authored but not executed. No reproduction, performance claim or physical qualification is made.


The scenario evaluator now contains all four scenario arms described in the Yao et al. benchmark: independent marginal negative control, empirical residual bootstrap, Schaake shuffle and a Gaussian copula. The copula arm uses rank-normal residual dependence, raw residual/evidence retention with score and correlation recomputation, per-residual model/regime/clock/time-origin contracts, a frozen regularized correlation matrix, deterministic Cholesky sampling and the current forecast's marginal quantile functions. It fails closed on low support, constant dimensions, invalid matrices and future/overlapping residuals. The implementation is in a separate draft stacked on PR #407; 36 tests are authored but not executed. This is methodological parity, not a completed paper reproduction or measured improvement.


## Paired, cluster-aware scenario decision benchmark

A new Rust \`scenario_tournament\` layer now compares independent marginal sampling, Gaussian copula, empirical residual bootstrap and Schaake shuffle on the exact same forecast origins and measured horizons. It rejects comparisons that change the model, clock, workload, measurement evidence, horizon, scenario count or reserve/shortfall ratio. Bootstrap and Schaake selected residual records must also match the same complete historical residual pool used by the Gaussian copula, including raw residual vectors and origin/time/source evidence. Forecast-origin timestamps are checked against the preregistered episode windows.

The statistical unit is a fixed-exposure operating cluster. Episodes inside a cluster may be dependent; the benchmark aggregates their paired cost outcomes before inference. It uses an exact one-sided Clopper–Pearson lower bound over independent clusters and Bonferroni-adjusted confidence across required strata. Overall pooled cost comparisons are descriptive only.

The internal research target is at least 7.4% lower normalized reserve cost than the independent-marginal negative control while satisfying the same preregistered shortfall ceiling and cluster-level reliability requirement. The published 7.4–10.6% study result motivates the target; no improvement has been demonstrated by Luminous.

See [Paired, Cluster-Aware Scenario Decision Tournament](./SCENARIO_DECISION_TOURNAMENT_2026-11-11.md). Twenty-one synthetic Rust tests are authored but not executed; no compilation, physical measurement or grid reserve qualification is claimed.
