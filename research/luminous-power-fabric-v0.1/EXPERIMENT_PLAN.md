# PEBB-LV-01 experiment and execution plan v0.1

**Parent:** [Issue #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Status:** proposed staged plan; no tests or hardware have been run  
**Safety boundary:** low-voltage, current-limited research only during phase 0

## Principle

Do not wait for unrelated hosted runners to begin research. Work can proceed in parallel on literature,
requirements, model review, measurement schemas, BOM scouting and deterministic offline analysis.
Physical energization is a separate gate and must not be inferred from software progress.

## E0 — requirements, safety and evidence contract

**Outputs**
- Requirements-to-test traceability matrix for PEBB-LV-01.
- Initial hazard analysis: electric shock, arc/thermal burn, fire, stored energy, reverse current,
  polarity error, overvoltage, loss of cooling, insulation failure, unexpected restart and
  measurement/probe error.
- State model and safe-state invariants.
- Preliminary insulation coordination and standards applicability matrix.
- Instrument and uncertainty plan.
- Measurement data contract and raw-data retention policy.

**Exit gate**
A qualified power-electronics engineer reviews the intended use, component ratings, fault energy,
protection, enclosure, discharge path, isolation and lab procedure before any build is energized.

## E1 — literature and topology screen

**Parallel work that does not require runners**
1. Extract topology variants and reported conditions from the 2026 IEEE SST PEBB paper.
2. Build DAB and CLLC loss trees with conduction, switching, gate-drive, magnetic, capacitor,
   interconnect and auxiliary terms.
3. Identify realistic 36–60 V semiconductor candidates from manufacturer datasheets. Compare
   silicon MOSFET, SiC and GaN without assuming a wide-bandgap device always wins.
4. Create transformer / resonant component trade-space with material, core loss, winding loss,
   thermal and insulation considerations.
5. Compare the same voltage/power points in both directions, including light-load and worst-case
   voltage ratio.
6. Create BOM / lifecycle / second-source / local availability table.

**Model requirements**
- Include Rds(on) versus temperature and gate conditions.
- Include switching energy versus voltage/current/temperature where datasheets allow.
- Include dead-time, output-capacitance and reverse-conduction effects.
- Include transformer core and copper loss with provenance and uncertainty.
- Include capacitor ESR and ripple current.
- Include conductor, connector and PCB parasitics.
- Include control, gate-drive and cooling auxiliary power.
- Preserve model version, parameter source and uncertainty per component.

**Exit gate**
Down-select a preferred topology and a credible alternative using the same boundary and a
multi-objective decision table. The chosen option must explain why it wins beyond peak efficiency.

## E2 — model and numerical verification

- Construct the first non-ideal switching/converter model with a documented solver and timestep.
- Verify steady-state energy balance and power sign convention.
- Test parameter bounds, extreme voltage ratios, startup, light-load and bidirectional operation.
- Compare average models against switching models where both are appropriate.
- Check thermal-model sensitivity to switching, copper, magnetic and auxiliary losses.
- Define model-to-bench correlation metrics before prototype measurements exist.
- Store input parameters, model source digest, solver version, convergence settings and output digest.

**Negative controls**
- Ideal switch / ideal transformer assumptions must be visibly marked and cannot qualify a result.
- Invalid or missing loss parameters must produce UNKNOWN/invalid model status, not zero loss.
- Time-step or solver non-convergence must not produce a performance pass.
- Model parameters from another hardware revision cannot be silently reused.

**Exit gate**
The model passes its own numerical and conservation checks and is explicitly labelled **unvalidated
against physical hardware** until correlation data exist.

## E3 — low-voltage prototype and controlled bench characterization

**Equipment (to be specified and reviewed)**
- Current-limited, appropriately isolated DC laboratory source(s).
- DC electronic load or controlled bidirectional source/load for each port as required.
- Calibrated power analyzer or voltage/current instruments with adequate bandwidth/accuracy.
- Oscilloscope, rated differential probes and current probes appropriate to actual switching edges.
- Temperature sensors / thermal camera with documented calibration and emissivity assumptions.
- DC-rated fuses, contactors/disconnects, guarded enclosure, interlocks and safe discharge path.
- Emergency isolation and reviewed operating procedure.

No equipment is presumed to be on hand or qualified. Exact models, ratings, calibration and working
limits must be recorded before use.

**Bring-up order**
1. Inspect board and BOM; verify polarity, clearances, fasteners, sensor orientation and no shorts.
2. Test control and protection on an unpowered or safely isolated setup where possible.
3. Verify gate-driver disabled default and hardware inhibit.
4. Start with low bus voltage, low current limit and low duty / low power.
5. Validate current-sense polarity, scaling, offsets and independent comparator trip.
6. Validate soft start, output pre-charge/discharge and fault latch.
7. Increase power in predefined increments only after thermal and waveform review.
8. Run steady-state map at 36/48/60 V and the declared load fractions in both directions.
9. Run transient tests only after steady-state stability is established.
10. Preserve raw captures and analysis digests for every run.

**No battery packs, public mains, PV strings, utility connections or medium-voltage sources in E3.**

## E4 — fault and resilience characterization

Only execute a fault injection when the specific fault, available energy, containment, disconnect,
instrumentation, abort condition and operator roles have been reviewed.

Cases include:
- Port undervoltage and overvoltage.
- Current limit and controlled overload.
- Overtemperature and loss of cooling.
- Sensor open, short, saturation, offset drift and wrong polarity.
- Controller reset, watchdog timeout and firmware mismatch.
- Gate-driver fault and gate-enable stuck-at negative control.
- Communication loss, malformed command, stale command and replay.
- Output disconnection and controlled reversal.
- Protection latch, safe discharge and authorized re-arm.
- Unknown measurement/actuation outcome: no additional energization until reconciliation.

A hard short or high-energy fault is not a default test case. It needs a separate engineered test
fixture and approved procedure; never create an uncontrolled fault on a bench.

## E5 — mission-profile and repairability

- Define representative usage profiles before accelerated testing.
- Identify component-level wear-out mechanisms and random failures separately.
- Develop power-cycling and thermal-cycling test plans from measured temperature swings and
  component limits.
- Include humidity, vibration, connector mating, fan life and storage where the product requires.
- Test replaceable power-stage, controller, sensor and cooling module service procedures.
- Re-run affected electrical, thermal, insulation, protection and firmware tests after repair.
- Model lifetime with uncertainty and disclose acceleration assumptions; no direct extrapolation
  from a small cycle count to decades of service.

## E6 — microgrid integration and PHIL

After the DC/DC block is characterized, develop a separate AC/DC interface programme:
- Grid-following and grid-forming inverter architectures.
- Protection and isolation for AC and DC sections.
- Grid-connected / islanded transitions and black start where claimed.
- Weak-grid, voltage/frequency disturbance and multi-vendor interoperability.
- Controller-HIL first, then power-HIL in a qualified specialist laboratory.
- Correlate models against commercially available reference inverters and measured equipment.

National Laboratory of the Rockies describes MW-scale PHIL capabilities; we should eventually
partner with a suitably equipped laboratory rather than attempt utility-scale fault testing in-house.

## E7 — commercial and certification readiness

- Product-specific standards mapping and jurisdictional requirements.
- Design for manufacturing and assembly, supplier quality, alternate parts and traceability.
- Accredited laboratory pre-compliance and type testing.
- Installation, commissioning, maintenance, repair, end-of-life and safety documentation.
- Cybersecurity update policy and defined support lifetime.
- Warranty reserve, service model, spare parts and product liability review.
- Pilot deployment only after certifications/permissions and site engineering are complete.

## First 10 tasks that can start immediately

| ID | Task | Primary system | Required evidence |
|---|---|---|---|
| PF-001 | Freeze initial use case and requirement traceability | Symthaea + engineering review | Versioned requirements matrix |
| PF-002 | Extract 2026 PEBB topology comparisons and operating conditions | Symthaea | Literature evidence table |
| PF-003 | Build DAB loss tree and parameter provenance | Symtropy | Non-ideal model specification |
| PF-004 | Build CLLC loss tree and parameter provenance | Symtropy | Matched candidate model specification |
| PF-005 | Create semiconductor candidate/BOM/lifecycle matrix | Symthaea + procurement | Datasheet-backed part ledger |
| PF-006 | Define measurement CSV and calibration record contract | Mycelix + engineering | Versioned schema and examples |
| PF-007 | Implement offline measurement auditor (efficiency, ripple, temperatures, sign checks) | Rust tooling | Unit tests, fixtures and exact build receipt |
| PF-008 | Create physical packaging and replaceable-module envelope | Sol-Atlas | Revisioned geometry/BOM |
| PF-009 | Create fault tree, FMEA and safe-state transition table | Symtropy + independent reviewer | Reviewed safety case draft |
| PF-010 | Establish model-to-bench correlation protocol | Symtropy | Preregistered correlation plan |

The first code increment should be the offline measurement auditor, not an autonomous hardware-control
agent. It will consume read-only measurement records and fail closed on missing/ambiguous data.

## Claim ceiling

E0–E2 work can produce research, specified interfaces, models and software-checked outputs. Only
E3/E4 can produce bench measurements. E5 can support limited reliability screening. E6 requires
qualified HIL facilities. E7 requires applicable independent conformity assessment and jurisdictional
approval. A later stage never retroactively upgrades an earlier simulation or synthetic fixture.
