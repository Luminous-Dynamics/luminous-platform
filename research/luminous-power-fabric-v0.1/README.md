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


## Portable field-power systems and rugged industrial deployment

[Issue #413](https://github.com/Luminous-Dynamics/luminous-platform/issues/413) and
[PR #414](https://github.com/Luminous-Dynamics/luminous-platform/pull/414) extend the Power Fabric
PEBB work into portable service nodes, towable/mobile hybrid power systems and later plant-support
systems. The product architecture and benchmark boundary are documented in
[FIELD_POWER_PLATFORM.md](./FIELD_POWER_PLATFORM.md). A rights-aware source/load duty-cycle schema
and synthetic construction profile are in `schemas/field-mission-profile-v0.1.schema.json` and
`schemas/field-mission-profile.synthetic.json`.

## DC branch power protection

[Issue #415](https://github.com/Luminous-Dynamics/luminous-platform/issues/415) studies a 48 V-class
solid-state branch controller. The design basis compares integrated eFuses, external-FET controllers,
discrete SSPCs, conventional fuse/mechanical disconnect and hybrid semiconductor/mechanical
interruption. See [LV_DC_BRANCH_CONTROLLER_DESIGN_BASIS.md](./LV_DC_BRANCH_CONTROLLER_DESIGN_BASIS.md).

## Evidence-gated capability envelope

[Issue #416](https://github.com/Luminous-Dynamics/luminous-platform/issues/416) introduces a typed,
read-only Rust evaluator for current, direction-specific continuous-power capability. It binds
configuration identity, qualification expiry, calibration/sensor status, protection readiness,
telemetry freshness, thermal derating and conservative uncertainty. It does not control hardware or
replace independent protection. See [CAPABILITY_ENVELOPE_DESIGN.md](./CAPABILITY_ENVELOPE_DESIGN.md),
the capability JSON schema and synthetic fixture under `schemas/`, and the measurement-audit crate.

The next multi-port step is common-cause-aware capability and recovery analysis in
[issue #417](https://github.com/Luminous-Dynamics/luminous-platform/issues/417).

## Standards applicability

The dated standards and conformity workstream is maintained in
[STANDARDS_APPLICABILITY_RESEARCH.md](./STANDARDS_APPLICABILITY_RESEARCH.md). It distinguishes
converter safety, breaker/isolator claims, construction-site assemblies, site installations,
battery/ESS testing, environmental qualification and South African NRCS product-category review.
A standards reference is not a compliance or certification claim.


## Common-cause-aware multi-port capability

[Issues #417 and #418](https://github.com/Luminous-Dynamics/luminous-platform/issues/417) now have a deterministic
Rust composition slice in `tools/measurement-audit/src/multi_port_capability.rs`. It composes already
qualified parallel route bottlenecks, automatically evaluates component and shared failure-domain
losses, supports explicit compound contingencies, and applies shared throughput constraints for
declared buses/feeders/cooling groups. Shared capacity is recomputed after each contingency; partially
overlapping non-identical shared-capacity groups are rejected. The typed input/schema and synthetic
shared-bus fixture are under `schemas/multi-path-capability-*`, with the model and benchmark boundary
in [MULTI_PORT_CAPABILITY_DESIGN.md](./MULTI_PORT_CAPABILITY_DESIGN.md).

This remains a declared-contingency and shared-throughput arithmetic screen. It does not discover
missing common causes, solve electrical networks, calculate fault current or protection selectivity,
estimate failure probabilities, model electrochemical/thermal storage dynamics, simulate source ramp
or transients, solve recovery/requalification, or authorize switching. The separate mission-energy
screen accounts for interval energy and reserve only; dynamic storage and recovery remain owner-native
model and physical-test work.

## Time-dependent mission energy and storage reserve

[Issue #422](https://github.com/Luminous-Dynamics/luminous-platform/issues/422) adds a read-only
mission-energy evaluator in `tools/measurement-audit/src/mission_energy.rs`. It composes
Symtropy's qualified interval-average source capability with a single aggregate storage envelope,
subtracts supplied source uncertainty bounds, applies charge/discharge power and efficiency limits,
tracks SOC, and enforces an end-of-mission reserve. The schema and synthetic two-segment hybrid
fixture are under `schemas/mission-energy-envelope-*`; the design, equations and benchmark tiers
are in [MISSION_ENERGY_ENVELOPE.md](./MISSION_ENERGY_ENVELOPE.md).

This is an interval-energy accounting screen, not a dispatch controller, electrochemical/thermal
storage model, generator/inverter transient simulator, fuel model, network solver or physical
qualification. Source ramps/start delays must be reflected in owner-qualified interval-average source
capability; sub-second events remain separate dynamic and physical-test obligations. The evaluator
retains source and storage evidence/configuration lineage and fails closed on global configuration
mismatch or expired mission qualification.

## Lifecycle service accounting and observed storage health

[Issue #427](https://github.com/Luminous-Dynamics/luminous-platform/issues/427) introduces
`tools/measurement-audit/src/lifecycle_service_cost.rs`, a read-only lifecycle accountant.
It calculates matched cost per delivered critical kWh and per completed mission from integer
minor-currency ledgers, requires unique economic-event IDs, unions overlapping system-unavailable
repair intervals, and tracks labour, requalification and component replacement identity.

For storage, the accountant reports capacity and discharge-power retention only from repeated
independent, calibrated measurements of the same component and unchanged configuration, with
monotonic throughput/calendar/EFC history and timebase references. It never fits an ageing curve or
predicts RUL. A complete repair-history record is mandatory: an empty repair list without evidence
that the maintenance history is complete is not evidence of zero repairs.

The schema and explicitly synthetic comparator fixture are under `schemas/lifecycle-service-accounting-*`;
equations, ownership and claim ceilings are in
[LIFECYCLE_SERVICE_ACCOUNTING.md](./LIFECYCLE_SERVICE_ACCOUNTING.md). Synthetic records demonstrate
the arithmetic but are deliberately ineligible for physical lifecycle claims.

## Matched-service industry benchmark

[Issue #419](https://github.com/Luminous-Dynamics/luminous-platform/issues/419) and
[INDUSTRY_BENCHMARK_PROGRAMME.md](./INDUSTRY_BENCHMARK_PROGRAMME.md) pin the Cat XES120,
Atlas Copco ZBC, ABB Infinitus and TI TIDA-050090 vendor baselines and define component,
source-to-load, useful-service and resilience benchmark tiers. The benchmark reuses the existing
field-mission-profile contract and composes Symthaea, Symtropy, Sol-Atlas and Mycelix owners.
Vendor specifications remain separate from independent physical measurements and accredited
qualification evidence.

## Current limitations

This package contains analytical screening models and offline data-audit tools; it does **not**
contain a validated non-ideal switching model, a fabricated PCB, a power-stage prototype, calibrated
physical measurements, HIL correlation, production tooling or safety certification. The DAB and
CLLC models remain bounded by their documented ideal/FHA/equation assumptions. No result is a
physical ZVS guarantee or permission to energize hardware. High-voltage and grid-connected product
families remain architecture/research tracks only. Hardware energization remains gated on qualified
engineering review, fault/energy analysis, suitable laboratory containment and a reviewed procedure.


## Uncertainty-qualified lifecycle service frontier

[Issue #461](https://github.com/Luminous-Dynamics/luminous-platform/issues/461) and draft [PR #462](https://github.com/Luminous-Dynamics/luminous-platform/pull/462) add a read-only decision layer over `LifecycleScenarioReport`. It applies non-compensatory critical-service and evidence gates, then reports central and interval-robust Pareto frontiers across lifecycle cost, unserved critical service, unavailable hours, replacements and (where comparable) observed storage retention. It does not create ageing forecasts or statistical confidence intervals.


## Paired cluster bootstrap

Draft [PR #463](https://github.com/Luminous-Dynamics/luminous-platform/pull/463) adds deterministic paired percentile-bootstrap summaries over pre-aggregated mission/site/event clusters. It requires exact cluster and scenario-seed pairing, retains failed missions and unserved energy, and rejects zero-delivery draws instead of censoring them. Cluster-independence evidence is caller-supplied and is not proven by this code.


## Preregistered superiority gate

Draft [PR #464](https://github.com/Luminous-Dynamics/luminous-platform/pull/464), tracked by issue #466, binds the lifecycle frontier and paired cluster bootstrap to one preregistered primary metric, candidate family, Bonferroni-adjusted confidence and conservative paired metrology-error bounds. It distinguishes established, no-preregistered-superiority, inconclusive and ineligible; it makes no equivalence or field-superiority claim.
