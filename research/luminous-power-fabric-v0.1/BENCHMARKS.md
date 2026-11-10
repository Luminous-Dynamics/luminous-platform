# Luminous Power Fabric benchmark protocol v0.1

**Parent:** [Issue #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Status:** metric and target proposal; no Luminous hardware results exist  
**Rule:** report separate metrics; no blended score can hide a safety, reliability or efficiency regression.

## 1. Competitive reference points

| Reference | Publicly reported capability | Correct use in comparison | Limitation |
|---|---|---|---|
| IEEE Open Journal of Power Electronics, 3 Jun 2026, DOI 10.1109/OJPEL.2026.3699737 | Three experimentally validated DC/DC PEBB topologies for modular SST; 98.55% reported peak efficiency | Literature anchor for modular isolated DC/DC topology research | Not a matched 48 V / 500 W product; operating point, boundaries and topology differ |
| IEEE Access, 23 Apr 2026, DOI 10.1109/ACCESS.2026.3686835 | Experimentally validated 25 kW dual DAB–CLLC architecture; 200–1000 V operating range; 98.37% peak charging / 97.82% peak discharging; reconfigurable series/parallel module arrangement | Research anchor for range-adaptive, composable PEBBs | EV charging power/voltage class; not a matched 48 V / 500 W benchmark |
| ELSİS BIDIR Module (official product page / datasheet) | 500 W bidirectional low-voltage buck-boost; 9–60 V input; 3.3–48 V adjustable output; vendor states >97% efficiency and >97% at full rated load | Closest functional commercial 500 W bidirectional low-voltage baseline | Public spec does not state galvanic isolation or fully define efficiency test conditions; vendor result, not independent lab result |
| MPS MPC10106-54-0500 (Q4 2024 brochure) | 500 W continuous, 40–60 V input, 8:1 non-isolated LLC, 2,634 W/in³ stated density and 97.8% peak efficiency | 48 V data-centre density / point-of-load conversion reference | Not bidirectional or isolated; fixed-ratio low-voltage output |
| MPS MPC10106-B-54-0500 (2025 brochure) | 500 W-class, 40–60 V input, 8:1 non-isolated LLC, 3,019 W/in³ and 97.65% peak efficiency | Newer 48 V data-centre power-density reference | Different SKU / brochure from the 2024 module; not bidirectional or isolated |
| Wang et al., 500 W bidirectional CLLC (June 2026) | ZVS prototype; 97.6% peak forward-transfer efficiency; FHA and soft-switching-constrained resonant design | Same-rated-power laboratory topology benchmark | Abstract does not establish a matched reverse-flow or full-envelope efficiency map; not a direct PEBB-LV-01 result |
| Vicor V48A48C500 family | 500 W, 48 V output, 36–75 V input, 3 kV isolation, 89.7% typical efficiency at nominal input / 75% load / 25 °C per family data | Adjacent isolated-module reference | Not bidirectional; verify exact SKU and revision; not matched to PEBB-LV-01 |
| ABB SACE Infinitus | Up to 2,500 A / 1,250 V DC; interruption down to 25 μs; IEC 60947-2 certified; combines solid-state interruption and electromechanical isolation | Commercial benchmark for mature solid-state DC protection | Different product, power class, thermal/cooling design and application; do not compare its interruption time to a converter |
| ESIG, Sep 2025, Testing the Performance of Grid-Forming Resources | Methods and metrics for voltage-source behavior of grid-forming resources | Future microgrid AC interface test-plan basis | Not a converter efficiency benchmark; tailor to system and grid conditions |
| National Laboratory of the Rockies PHIL | 1.08 MVA and 7 MVA grid simulators described for power hardware-in-the-loop | Future system-level hardware validation ambition | Facility capability is not a pass criterion for a 500 W DC/DC unit |
| IEC 62909-1:2025 | General and safety requirements for bidirectional grid-connected converters in its scope | Standards mapping for later relevant multi-port grid-connected products | Explicitly excludes several product categories including PV conversion, UPS and EV charging; scope must be checked |

A vendor's published maximum, a research-paper peak, and a measured Luminous result are different
evidence classes. Record the original boundary and never imply apples-to-apples equivalence without
matched conditions.


A second 2026 paper shows that modular reconfiguration can be more important than simply selecting
one resonant or bridge topology: its 25 kW dual DAB–CLLC design changes module series/parallel
arrangement to accommodate a 200–1000 V battery-side range. Treat this as a later architecture
research direction. PEBB-LV-01 has a narrow 36–60 V port envelope and should not inherit unnecessary
reconfiguration complexity.

**Target-setting correction:** ≥97% is a functional Rev-A target, not a claim to beat the industry.
The June 2026 500 W bidirectional CLLC study reports 97.6% peak forward-transfer efficiency. ELSİS
advertises >97% at full rated load current for its 500 W bidirectional buck-boost module, while the
ELSİS datasheet's cover says >95% and its detailed features say >97% at full rated load current.
MPS's newer 2025 brochure reports 97.65% peak for a separate 500 W non-isolated 48 V LLC module.
These figures have different boundaries and directions; none is a matched independent comparison.
See [INDUSTRY_BASELINE.md](./INDUSTRY_BASELINE.md).

The first competitive target should therefore be ≥98.0% at explicitly matched nominal operating
points **in both directions**, with a separately reported complete efficiency surface. This is a
proposed stretch objective, not a guaranteed result. Keep ≥97% as the functional gate until the
non-ideal model, full paper test conditions and uncertainty budget are reconciled.
ELSİS advertises >97% at full rated load for a 500 W bidirectional buck-boost module, and MPS
advertises 97.8% peak for a different 500 W non-isolated 48 V LLC stage. These are not matched to
our isolated DAB/CLLC target, but they show that headline efficiency alone is insufficient. The
competitive gate is a direct, uncertainty-aware comparison against the closest available baseline,
with isolation, direction, thermal and service requirements held explicit. See
[INDUSTRY_BASELINE.md](./INDUSTRY_BASELINE.md).

## 2. PEBB-LV-01 proposed target matrix

All values below are **proposed engineering targets**, not measured results, standards limits or
guarantees. Revisit them after the topology model and first lab data.

| Metric | Initial functional gate | Rev-A target | Stretch / research frontier |
|---|---|---|---|
| Bidirectional transfer | Stable low-power transfer in both directions | Full declared 36–60 V envelope at 500 W continuous target | Controlled power reversal across wider voltage-ratio range |
| Efficiency | Produce complete measured map; explain loss budget | ≥97.0% at nominal voltage / 50–100% load in both directions | ≥98.0% at nominated nominal point; improve light-load map |
| System boundary | Report converter and auxiliaries separately | Include controller, gate drive, fans and internal power supplies in system efficiency | Optimize total end-to-end energy across source, converter, distribution and load |
| Standby | Measure no-load and disabled draw | ≤3 W at declared measurement boundary | ≤1.5 W without weakening protection or diagnostics |
| Regulation | Stable bounded control | ±1% steady-state output voltage at declared points | Better transient regulation without raising switching stress |
| Ripple | Measure with fixed probe/bandwidth method | ≤1% p-p of nominal output voltage | Lower ripple without excessive filter size/loss |
| Transient response | Record step response and stability | Declare step profile, overshoot/undershoot and settling-time target before test | Better settling under both directions and input extremes |
| Thermal | Record temperatures and equilibrium | No component exceeds its reviewed derated temperature limit at 500 W / stated ambient | Lower hot-spot rise and cooling parasitic power |
| Power density | Report kW/L and kW/kg including enclosure/cooling | Establish baseline before setting a numeric target | Improve only without worsening reliability, serviceability or EMI |
| Protection | Demonstrate safe inhibit in reviewed low-energy fault tests | Zero unsafe gate-enable states in negative-control corpus | Selective fast protection and graceful fault containment |
| Repair | Document one replaceable module boundary | Replace power stage / controller / fan independently where feasible | Verify post-repair requalification in a versioned test plan |
| Reliability | Build a mission-profile model and fault tree | Screen power/thermal cycling on a documented sample | Independent lifetime correlation; no lifetime extrapolation without validated models |
| Evidence | Preserve test input and output bytes | Calibration, uncertainty, firmware, BOM and test conditions bound to every run | Independent offline verifier and external reproducibility |

### Higher-power bidirectional CLLC research anchors

The October 2026 4.8 kW PV-storage CLLC prototype reports 97.45% peak charging and 96.66% peak
discharging efficiency. It demonstrates that forward/reverse efficiency must be reported separately,
and that PFM timing resolution, gain monotonicity, inductive input impedance, Coss/dead-time commutation
and thermal design belong in the same design review. Its 40–60 V battery side, 360–540 V DC bus,
9:1 transformer and high-frequency controller are not a direct match to the 48 V / 500 W PEBB.

A separate 2 kW T-type impedance-network CLLC study validates forward/reverse ZVS for its design
and reports 95.7% maximum forward efficiency (95.2% at rated forward load) and 95.4% reverse
efficiency. Its 150–300 V input, 75–167 V output, 115 kHz resonance and 75–150 kHz switching range
are not a 48 V / 500 W match. Use it as a design-method reference, not an efficiency SOTA claim.
Both papers reinforce the importance of a state-constrained feasibility region rather than selecting
a tank from gain alone.

### Full-profile and multi-objective research frontiers

- A 7.3 kW CLLLC-RDAB prototype reports 97.1–97.8% over its entire 200–500 V charging profile at
  400 V input / 100 kHz, with nearly identical reverse efficiency. This raises the benchmark beyond
  a peak point: evaluate efficiency weighted by the actual mission profile and report forward /
  reverse symmetry. It is not a 500 W low-voltage match.
- A 1 kW DAB study reports >95.9% peak efficiency while optimizing ZVS range, rating utilization and
  reactive power using MOPSO + TPS. We should add these as distinct Pareto metrics, not only peak
  efficiency.
- A 2026 physics-informed SPS DAB study shows FHA error reaches ~7.9% at 15° and 27.3% at 90° in its
  baseline. Our DAB model should report approximation error and switch to exact piecewise current /
  power evaluation where FHA exceeds the declared error budget.

For PEBB-LV-01, proposed next-stage research targets (not guarantees):
- ≥97.5% mission-weighted full-map system efficiency in each direction once the topology and
  measurement boundary are fixed; ≥98.0% at selected nominal points as a stretch objective.
- 100% of the declared operating map must satisfy hard voltage/current/thermal/insulation constraints.
- Report ZVS-polarity coverage separately from verified device-level ZVS coverage; no percentage is
  allowed to imply measured commutation unless switch waveforms support it.
- Publish RMS and peak current, reactive VA, rating utilization, auxiliary power and uncertainty at
  every map point.
- Compare loss-per-kWh and lifetime service cost, not just peak efficiency.

### 500 W loss-budget comparison

At 500 W output, the loss equivalents are:
- 97.0%: 15.464 W maximum total loss.
- 97.6%: 12.295 W maximum total loss.
- 98.0%: 10.204 W maximum total loss.
- 98.5%: 7.614 W maximum total loss.

This turns the 98% objective into an explicit approximately 2.09 W reduction from the loss equivalent
of a 97.6% result. These are arithmetic allowances, not model outputs. The new
[loss-budget engine and topology plan](./LOSS_BUDGET_AND_MODEL_PLAN.md) preserves term provenance and
uncertainty rather than inventing component loss values.

### Efficiency targets and why they are deliberately staged

The 98.55% IEEE result is a useful modular SST PEBB literature anchor. It is not a promise that a
small low-voltage 500 W converter can or should reproduce the same number. Low-voltage ports imply
high current; conduction loss, copper, interconnect and auxiliary draw matter. We will therefore
report the full efficiency map and target ≥97.0% at selected high-load nominal points first, with
98.0% as a stretch target after real loss data exists.

At 500 W output, 97% efficiency corresponds to about 15.5 W of converter loss, while 98% corresponds
to about 10.2 W. This arithmetic illustrates why heat removal and measurement uncertainty matter;
it is not a predicted result.

## 3. Operating-point matrix

For each topology and prototype revision, test in both directions:

- Port A and B setpoints: 36 V, 48 V and 60 V where the configuration permits.
- Load fractions: 10%, 25%, 50%, 75% and 100% of the declared continuous rating.
- Ambient: record actual ambient; nominal baseline 25 °C, additional temperatures only after the
  operating and safety envelope is reviewed.
- Cooling: passive and forced-air, with fan/auxiliary consumption included in system-level results.
- Thermal state: cold-start and steady-state equilibrium; define equilibrium as a temperature-rate
  threshold over a defined interval before running the measurement window.
- Repeats: at least three independently re-established runs per point for an initial characterization;
  use more units and samples for reliability/statistical claims.
- Transients: predeclare load steps, source steps, direction reversal and recovery criteria. Do not
  combine transient and steady-state samples into one efficiency number.
- Controls: test sensor disconnect/short, polarity mismatch, controller reset, watchdog expiry,
  communication loss, gate-driver fault, overcurrent, overvoltage, thermal trip and unresolved
  outcome. Fault tests require a separately approved energy/fault procedure.

This matrix is a proposed screening matrix, not a replacement for a product-specific standards
test programme.

## 4. Metric definitions

### Energy efficiency

Use simultaneous input/output measurement and integrate energy over the same steady-state
measurement window:

eta = 100 * E_out / E_in

E_in = integral(P_in dt); E_out = integral(P_out dt).

State current sign convention, voltage/current measurement location, sample rate, bandwidth,
integration rule, window duration and inclusion/exclusion of auxiliary loads. Report both:
- **Power-stage efficiency** (switching stage and magnetics boundary).
- **System efficiency** (including control, gate drive, fan and internal auxiliary supply).

Do not report a single value if the boundary or energy direction is ambiguous.

### Measurement uncertainty

Create a measurement uncertainty budget that includes analyzer accuracy, sensor/probe accuracy,
calibration status, phase/time alignment, sample rate, bandwidth, shunt/probe insertion effects,
temperature drift and integration. The proposed combined expanded uncertainty target is ≤0.2
percentage points for close efficiency comparisons. If this is not achieved, publish the actual
uncertainty and avoid a superiority claim smaller than the uncertainty interval.

### Power density

Report both:
- kW/L: continuous output power divided by external module volume, including enclosure and
  required cooling.
- kW/kg: continuous output power divided by the same complete module mass.

Publish a separate bare-board number only as supplementary metadata. Never compare an uncooled
bare PCB against an enclosed commercially rated converter.

### Thermal performance

Report ambient, airflow, orientation, heat-sink and interface material, fan speed/power, sensor
locations, calibration, equilibrium criterion, maximum measured temperatures and estimated
junction temperatures separately. Label junction temperatures as model estimates unless measured
with a validated method.

### Ripple and transients

Specify probe type, ground method, bandwidth limit, sampling and load/source step waveform.
Report ripple peak-to-peak and RMS separately. For transient tests report initial/final operating
point, step size/rate, minimum/maximum voltage/current, overshoot/undershoot, settling band,
settling time, protection events and whether any limit was reached.

### Reliability and lifetime

Use mission profiles with load, ambient, thermal cycling, switching cycles, humidity/contamination,
vibration, storage and service conditions. Maintain separate failure modes for semiconductors,
bond/wire/interconnect, solder, capacitors, magnetics, insulation/partial discharge, fans, connectors,
sensors and control firmware. Report test sample count, cycles, censored units, failures, confidence
bounds and model assumptions. Short screening tests do not establish a 20–30-year lifetime.

## 5. Fair industry-comparison rules

Every candidate and baseline record must pin:
- Exact device/model, hardware revision, BOM and semiconductor part numbers.
- Firmware, control parameters, switching frequency and build/source digest.
- Port voltage, current, power, flow direction and control mode.
- Ambient, altitude if relevant, airflow, enclosure, orientation and cooling.
- Instrument model/serial, calibration state, sensor/probe details, measurement boundary and
  uncertainty budget.
- Sample count, run order, warm-up and thermal-equilibrium method.
- Raw-data digest, analysis-tool revision, operator and timestamp.
- Applicable standard edition and test configuration where conformity is claimed.

Use the same equipment and measurement chain for direct head-to-head tests where possible. If an
external datasheet does not expose enough details to match, classify it as **not comparable**, retain
the vendor value as context and do not rank it.

No weighted composite "SOTA score". Safety, efficiency, reliability, power quality, repairability,
cost and environmental impact remain distinct dimensions.

## 6. Statistical and claim rules

- Freeze the scenario set and operating-point matrix before candidate testing.
- Retain every run, including failed, incomplete and invalid runs; state the exclusion reason.
- Pair baseline and candidate where input, workload, ambient and measurement chain can be matched.
- Report mean, median, min/max and uncertainty as appropriate; do not hide worst-case corners.
- Use independent test units for between-unit variability.
- Do not treat repeated samples from one device as independent production units.
- A zero-failure test does not imply zero risk. State sample size and an explicit confidence bound.
- Physical results must bind exact hardware and firmware. A simulation result never upgrades to a
  bench result merely because the same parameters were later used in a prototype.
- Better-than-SOTA requires matched conditions, same or better functionality, comparable evidence,
  independent reproduction and full disclosure of trade-offs.

## 7. Later grid / microgrid benchmark families

When the programme reaches AC/DC and grid-forming equipment, add—not substitute—the following
families:
- Grid-following/grid-forming capability and mode transition.
- Voltage-source behavior, frequency and voltage response, active/reactive power response.
- Black start and islanding transitions, where those functions are claimed.
- Voltage/frequency excursions, current limiting and overload behavior.
- Harmonics, flicker, unbalance, DC injection and power quality.
- Fault ride-through, protection coordination and selective isolation.
- Weak-grid stability, impedance interaction and multi-vendor parallel operation.
- PHIL / controller-HIL correlation and real-hardware safety.
- Jurisdiction-specific interconnection and conformity testing.

Use ESIG's 2025 grid-forming performance report and current utility/distributor requirements to
design the eventual protocol. Do not claim grid-forming capability from a DC/DC prototype.
