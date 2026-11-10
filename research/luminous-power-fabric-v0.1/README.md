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

## Current limitations

This package is a research specification. No hardware, PCB, firmware, converter model, benchmark
runner, thermal design or safety certification is claimed to exist. The first implementation work
is requirements, evidence tooling and a validated model; hardware energization remains gated by
specialist review and a controlled laboratory.
