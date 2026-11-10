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

## New topology-screening experiment package

### Literature reproduction before comparison

**CLLC-500W-2026:** obtain and examine the full article, supplement and any available raw data for
Wang et al. (DOI 10.1016/j.epsr.2026.112834). Extract exact primary/secondary nominal voltages,
gain range, rated-power boundary, switching-frequency range, forward/reverse test coverage, cooling,
instrumentation, efficiency measurement method, uncertainty and the operating point that produced
97.6%. Until these conditions are captured, only cite the reported peak forward result as literature
context.

**DAB-loss-2025:** reproduce the equation chain and parameter set from Wu et al. (DOI
10.1109/ICIEA65512.2025.11148703) if the full paper can be obtained. Its reported <5 W model/experiment
deviation is specific to that test setup; do not adopt it as our acceptance limit without matching
measurement uncertainty, power and hardware scale.

### Proposed common comparison matrix

For each non-ideal DAB and CLLC candidate, use the same nominal 500 W output boundary and run:

- Port A voltage: 36 V, 48 V and 60 V.
- Port B voltage: 36 V, 48 V and 60 V.
- Load fractions: 10%, 25%, 50%, 75% and 100%.
- Direction: A→B and B→A, recorded as separate runs.
- Thermal state: cold-start and declared steady-state equilibrium.
- Control: DAB SPS and at least one EPS/TPS variant; CLLC FHA-designed tank followed by time-domain
  verification and the proposed variable-frequency controller.
- Repeats: three independent model runs for deterministic software checks; physical sample counts
  must be set separately after model correlation and lab review.

This creates 90 voltage/load/direction operating points per candidate before modulation variants and
repeats. Invalid or infeasible points remain in the record with a reason; they are not quietly dropped
from the declared envelope. Points that exceed the declared current/thermal/semiconductor limit are
not energized; model-only infeasibility is preserved as a design rejection.

### Optimization objective

Use constrained multi-objective optimization, not a single weighted efficiency objective:
- Minimize integrated mission-profile conversion loss.
- Minimize peak/RMS semiconductor and magnetic current stress.
- Maximize ZVS/ZCS coverage and commutation margin.
- Respect junction, core, capacitor, connector, insulation and thermal limits.
- Minimize magnetics volume, total module mass and auxiliary consumption.
- Minimize BOM cost and single-source risk.
- Preserve reverse-transfer capability and bounded direction reversal.
- Preserve serviceability and independent fault containment.

Safety and physical limits are hard constraints. They cannot be traded against efficiency or cost.
A candidate only dominates another when it is no worse on the reported critical dimensions and
strictly better on at least one, with uncertainty and model validity shown.

### New CLLC study references and design rule

Add the October 2026 2 kW T-type impedance-network CLLC paper to the literature reproduction set.
Its key lesson is that FHA gain and input-impedance phase alone may omit internal resonant-state and
secondary modulation-reactance constraints. Reproduce its exact equivalent network before coding a
T-network evaluator. Add the 4.8 kW October 2026 PFM/HRPWM study as a separate design-reference case:
frequency quantization, register update synchronization, Coss and dead-time must be part of any
future digital-control feasibility model. Neither result is a direct 500 W design recommendation.

### Model-correlation protocol

The loss budget should be generated from the upstream topology model and carry a source for each
term. Compare prediction to measurement separately for input power, output power, total loss,
efficiency, RMS currents, hot-spot temperatures and waveform metrics. Do not calibrate the model
and claim validation on the same single run: reserve independent operating points / units for
correlation assessment.

## First implementation tasks that can start immediately

| ID | Task | Primary system | Required evidence |
|---|---|---|---|
| PF-001 | Freeze initial use case and requirement traceability | Symthaea + engineering review | Versioned requirements matrix |
| PF-002 | Extract 2026 PEBB topology comparisons and operating conditions | Symthaea | Literature evidence table |
| PF-003 | Build DAB loss tree and parameter provenance | Symtropy | Non-ideal model specification |
| PF-004 | Build CLLC loss tree and parameter provenance | Symtropy | Matched candidate model specification |
| PF-005 | Create semiconductor candidate/BOM/lifecycle matrix | Symthaea + procurement | Datasheet-backed part ledger |
| PF-006 | Define measurement CSV and calibration record contract | Mycelix + engineering | Versioned schema and examples |
| PF-007 | Extend the read-only measurement auditor to bind CSV captures to manifest digests, calibration and measurement uncertainty | Rust tooling + Mycelix | Exact raw bytes, instrument identity, calibration chain and invalid/unknown status; current CSV arithmetic auditor is implemented |
| PF-008 | Create physical packaging and replaceable-module envelope | Sol-Atlas | Revisioned geometry/BOM |
| PF-009 | Create fault tree, FMEA and safe-state transition table | Symtropy + independent reviewer | Reviewed safety case draft |
| PF-010 | Establish model-to-bench correlation protocol | Symtropy | Preregistered correlation plan |
| PF-011 | Run formatting, unit tests, lint and schema validation for the offline Rust crate at an exact recorded head | Rust tooling | Exact-head receipts; authored tests are not passing evidence until actually executed |
| PF-012 | Extract the full 500 W CLLC paper's voltage, power, direction and measurement conditions | Symthaea literature work | Source-linked comparator record; unknown fields remain unknown |
| PF-013 | Reproduce a published DAB SPS/EPS loss model and its assumptions | Symtropy | Equations, parameter set, model revision and independent reproduction notes |
| PF-014 | Build matched non-ideal DAB/CLLC operating-point maps | Symtropy | Full 90-point-per-candidate map with infeasible points retained |
| PF-015 | Create the Pareto analysis and sensitivity report | Symthaea + Symtropy | Separate efficiency, stress, thermal, volume, cost and model-uncertainty surfaces |
| PF-016 | Independently validate the implemented DAB SPS exact-current/RMS, resistive-periodic, FHA-error, ZVS-polarity and thermal screens | Rust tooling + Symtropy | Hand-recomputed vectors, model assumptions and no physical ZVS claim |
| PF-017 | Review and extend the implemented CLLC FHA gain and parameter-region evaluator | Symtropy + power-electronics review | Equation-by-equation derivation, directional/load Q, symmetry and boundary fixtures; no physical ZVS claim |
| PF-018 | Bind DAB/CLLC model loss terms into the provenance-aware loss-budget record | Rust tooling + Symtropy | Exact model revision, term-source and uncertainty-preserving budget records |
| PF-019 | Independently verify the implemented symmetric CLLC FHA gain evaluator and direction-specific Q inputs | Rust tooling + power-electronics review | Hand-calculated reference values and load/direction derivation; tests authored but not executed |
| PF-020 | Validate sampled CLLC gain-curve classification against dense equation sweeps | Rust tooling + Symtropy | Preserve finite-grid claim ceiling; quantify grid sensitivity and keep continuous monotonicity unproven |
| PF-021 | Independently verify the implemented T-network FHA phasor equations and sign conventions | Rust tooling + power-electronics review | Y-parameter / input-impedance reference vectors and reviewed current/port convention |
| PF-024 | Review and validate the implemented physical CLLC L/C/n to T-network and VF-TPS mapping | Symtropy + power-electronics review | Non-unity turns-ratio capacitor-referral regression exists; confirm exact schematic and transformer convention |
| PF-025 | Sweep the implemented CLLC P/N state classifier over voltage/load/frequency/modulation points | Symtropy | Retain near-zero as UNKNOWN; store Y1/Y2/Y3/Xeq and symmetry assumptions per point |
| PF-026 | Independently reproduce VF-TPS secondary impedance and composed T-network results | Rust tooling + power-electronics review | Eq. (1) vectors and port conventions; implementation exists, tests authored but not executed |
| PF-027 | Sweep the implemented Eq. (10) low-frequency boost window over candidate tanks and controller ranges | Rust tooling + Symtropy | Keep infeasible intersections and symmetry failures visible; no ZVS promotion |
| PF-032 | Validate the composite PNNN state screen across a declared CLLC operating-point matrix | Rust tooling + Symtropy | Physical L/C/n mapping, direction-specific VF-TPS impedance, symmetry and sign-state record for every point |
| PF-033 | Integrate and review CLLC FHA gain requirement with implemented state / CMP / unity-gain / maximum-gain analytical gates | Symtropy + power-electronics review | Keep gain, state, primary-side analytical ZVS and device commutation as separate report fields |
| PF-028 | Independently verify the implemented paper-derived constant-power Q, primary-side analytical ZVS limits and maximum-gain equations | Rust tooling + power-electronics review | Equation-derived reference vectors, M≥1 boost-domain precondition and no physical-ZVS claim |
| PF-029 | Reproduce the 500 W fixed-frequency LLC / bridge-leg-voltage-control topology paper | Symthaea + Symtropy | Source-linked topology, loss tree, control degrees of freedom and matched test conditions |
| PF-030 | Map 800 VDC rack/sidecar architecture, voltage interfaces and protection coordination requirements | Symthaea + Sol-Atlas + Symtropy | Requirements and standards routing matrix; no high-voltage bench work |
| PF-031 | Define a modular high-voltage DC/DC building-block contract and interoperability tests | Product architecture | Versioned electrical, mechanical, thermal, protection, service and evidence interfaces |
| PF-037 | Build whole-facility energy accounting across AC/DC conversion stages, cooling, storage and distribution | Symtropy | Explicit mission profile and matched energy-flow model; no efficiency claim before validation |
| PF-038 | Compare lossless DAB SPS model to resistive periodic RL solver as Req→0 | Rust tooling + Symtropy | Reference vectors for power, RMS current, periodicity and energy-balance residual |
| PF-039 | Validate DAB mission-profile sweep accounting, direction handling and Pareto filtering against independent arithmetic | Rust tooling + reviewer | Every infeasible point retained; weighted energy/current/thermal/ZVS screens are model-based only |
| PF-040 | Add DAB switch-level Coss / dead-time commutation and device thermal models | Symtropy + power-electronics review | Device-specific datasheet-backed parameters and measured waveform correlation plan |
| PF-034 | Freeze a representative DAB mission profile and candidate parameter grid | Symthaea + engineering review | Immutable point IDs, weights, voltage referral, power direction and environmental assumptions |
| PF-035 | Run deterministic RL-SPS DAB candidate sweep and Pareto filter | Rust tooling + Symtropy | Every point retained with failures; weighted efficiency, delivery, current, analytic ZVS and thermal margins |
| PF-036 | Validate mission-sweep direction, weighting, loss accounting and Pareto behavior against independent arithmetic | Rust tooling + reviewer | Forward/reverse power-boundary checks, energy balance, dominant/ non-dominant fixtures |


| PF-022 | Add Coss/dead-time commutation-energy and PFM timing-resolution constraints | Symtropy + embedded-control review | Declared device, gate-drive and controller clock parameters; no physical safety claims |
| PF-023 | Correlate CLLC FHA and state-constrained model against independent time-domain switching runs | Symtropy | Held-out operating points, residuals and model-validity report

The first code increments are the read-only capture auditor, provenance-aware loss-budget engine,
and ideal DAB SPS power-transfer screen. They are all offline/read-only engineering tools, not a
converter controller, protection relay or authorization to energize hardware.

| PF-041 | Extract full 6 kW 800 V/50 V APEC LLC paper conditions and recreate its reported efficiency/density boundaries | Symthaea + Symtropy | Source-linked data sheet and reproduction notes; all missing conditions explicit |
| PF-042 | Reproduce the CPSS 800 V-to-50 V ISOP LLC architecture and cell-sharing assumptions | Symtropy | Cell model, turns-ratio / transformer mapping, voltage/current sharing and test boundary |
| PF-043 | Build matched 800 VDC 6 kW LLC / ISOP non-ideal loss and electrothermal comparisons | Symtropy | Full-load maps, component losses, thermal parasitics and model uncertainty; no physical claims |
| PF-044 | Model 800 VDC protection / pre-charge / discharge / selective cell isolation / startup interfaces | Symtropy + power-electronics review | Fault-energy model, safe-state contracts and OCP v1.1.0 traceability; no high-voltage test |
| PF-045 | Independently validate conversion-chain auditor and matched baseline/candidate comparisons | Rust tooling + Symtropy | Complete 8,760-hour profile, idle/standby, stage-loss closure, matched delivered energy/tariff/currency/carbon context; 17 tests authored, not run |
| PF-046 | Map OCP LVDC v1.1.0 requirements for protection, startup, hold-up, control/monitoring and EMC | Symthaea + Sol-Atlas | Requirement-to-interface matrix; distinguish OCP published work from unresolved workstream detail |
| PF-047 | Prepare specialist 800 V laboratory / independent test partner brief | Product architecture | Equipment ratings, procedures, calibration, containment, competence and evidence expectations |

## Claim ceiling

E0–E2 work can produce research, specified interfaces, models and software-checked outputs. Only
E3/E4 can produce bench measurements. E5 can support limited reliability screening. E6 requires
qualified HIL facilities. E7 requires applicable independent conformity assessment and jurisdictional
approval. A later stage never retroactively upgrades an earlier simulation or synthetic fixture.
