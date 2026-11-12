# PEBB-LV-01 preliminary hazard and failure-mode register

**Status:** preliminary qualitative register; requires review by a qualified power-electronics / electrical-safety engineer before any hardware build or test.  
**Scope:** proposed 36–60 V DC, 500 W laboratory demonstrator only.  
**Not a safety certification, complete FMEA or authorization to energize hardware.**

## Risk-handling rule

Do not assign pseudo-precise RPN values before the operating environment, components, fault energy
and detection coverage are known. Record severity, mechanism, prevention, detection, containment,
test requirement and residual uncertainty separately. Any unreviewed high-consequence hazard
blocks energization.

| Failure / hazard | Credible mechanism | Primary prevention / design control | Detection / evidence | Required disposition |
|---|---|---|---|---|
| Port short / overcurrent | Load fault, wiring error, failed semiconductor, control saturation | Current-limited source; coordinated DC-rated fuse/protection; independent hardware current limit; bounded conductor/fault-energy design | Independent current sensor/comparator; captured trip waveform; fault-energy calculation | Inhibit gates, isolate safely, latch; no automatic restart until state is reconciled |
| Reverse polarity | Miswired port, connector mismatch, maintenance error | Keyed connectors, polarity marking, input protection sized by fault analysis | Pre-energization inspection; polarity negative control | Prevent energization; no software-only override |
| Port overvoltage | Incorrect source, loop instability, regenerative load | Independent OVP path; rated components; safe energy path | Independent voltage comparator and recorded trip test | Hardware inhibit and safe discharge |
| Stored-energy hazard | Capacitors or external source remain energized after shutdown | Bleeder/discharge path; guarded terminals; defined wait/verify procedure; DC-rated disconnect | Independently measured residual voltage after shutdown | Treat unknown residual voltage as energized; block service |
| Semiconductor thermal runaway | Excess conduction/switching loss, poor thermal interface, blocked airflow | Derating, thermal design, current limit, serviceable cooling | Multiple temperature sensors, thermal model and calibrated measurement | Inhibit before component limit; latch and inspect |
| Loss of cooling | Fan failure, blocked intake, pump failure if liquid cooled | Passive-safe thermal margin where feasible; fan tach/current monitoring; conservative derating | Fan fault injection and thermal rise test | Reduce power or inhibit according to reviewed safe state |
| Gate-driver fault | UVLO, desaturation, isolation failure, stuck output, Miller turn-on | Qualified gate driver, hardware interlocks, layout, gate resistors/clamps and independent disable | Gate waveform and fault-status capture | Hardware inhibit; fail-safe default must be disabled |
| Controller crash / reset | Brownout, firmware defect, watchdog failure | Gate-enable defaults disabled; external watchdog; bounded state machine | Reset/watchdog negative controls | No unintended pulse or automatic restart |
| Sensor open/short/drift | Connector failure, ADC saturation, offset drift, wiring fault | Range plausibility, independent comparator where appropriate, diagnostics | Open/short/offset injection with known reference | Missing/invalid sensor data is not interpreted as safe |
| Power-flow reversal transient | Sign error, unstable control handoff, wrong state transition | Ramp to zero transfer; explicit state transition; current limit | Bidirectional load/source step capture | Reject reversal unless prior transfer is bounded and state is known |
| Isolation breakdown / partial discharge | Incorrect creepage/clearance, voids, dv/dt, contamination, thermal aging | Standards-derived insulation coordination, suitable materials and layout, controlled manufacturing | Design review; accredited dielectric/PD tests when applicable | No isolation claim until appropriate physical qualification |
| EMI / sensor corruption | High dv/dt/di/dt coupling into measurements or control | Layout segregation, shielding, filtering, return-path design | Conducted/radiated pre-compliance and injected-interference tests | Fail safe; no unexplained instability |
| Connector / busbar overheating | Poor crimp/torque, undersized conductor, corrosion, repeated mating | Rated connectors, torque/process control, strain relief and contact-resistance limits | Thermal imaging and millivolt-drop measurement | Derate/inhibit; repair and requalify affected joints |
| Communications loss / stale command | Network interruption, replay, delayed command | Local control; command freshness and sequence; no network dependency for protection | Disconnect/replay/stale-message tests | Local safe operation continues; remote control path cannot bypass limits |
| Firmware / configuration mismatch | Wrong board image, rollback, configuration drift | Signed versioned builds, hardware compatibility manifest, bounded configuration | Image digest and compatibility check | Inhibit start; preserve evidence; no silent downgrade |
| Measurement error | Wrong probe, polarity, range, calibration or time alignment | Measurement procedure; instrument suitability; calibration and channel mapping | Independent cross-check, polarity checks and uncertainty budget | Run invalid/unknown; do not publish efficiency result |
| Unresolved actuation / test outcome | Controller or logger failure leaves device state uncertain | Explicit attempt identity, local state reconciliation and operator verification | Independent power/current/voltage observation | No new energization until reconciliation |
| Unsafe maintenance / hot swap | Human contacts energized or charged assembly | Lockout/tagout procedure, physical disconnect, discharge indicator and verification | Documented service checklist and independent voltage check | Service is blocked until safe-state evidence is available |

## Required negative controls

Before any power-stage test, demonstrate with a reviewed low-energy setup:

1. Gate outputs remain disabled through MCU reset and power sequencing.
2. Watchdog expiry produces a hardware inhibit.
3. Sensor open, short, saturation and wrong polarity cannot authorize increased power.
4. Communications loss and malformed/stale commands do not bypass local limits.
5. Overcurrent, overvoltage and thermal trip signals produce the intended latched response.
6. Fault reset cannot occur while the cause or residual energy remains unknown.
7. A failed or unavailable data logger does not keep the converter energized.
8. An instrument sign/timebase mismatch makes the measurement invalid rather than a pass.

## Review questions before energization

- What are the maximum available fault current and stored energy at each port?
- What is the interrupting rating and DC polarity/direction capability of each protective device?
- What is the measured discharge time and how is residual voltage independently verified?
- Are the isolation working voltage, overvoltage category, pollution degree, material group, altitude,
  creepage, clearance and impulse withstand requirements defined for the actual use case?
- Is the protection independent of the MCU and of communications?
- Can the device re-energize after a brownout, firmware reset, fault clear or restored communication?
- How do probe loading, common-mode range, bandwidth and ground connection affect test safety?
- Which repairs invalidate the insulation, thermal, EMI, protection and power-performance evidence?

## Claim ceiling

This register is a preliminary design aid. It does not demonstrate hazard elimination, compliance,
functional safety, safe operation, protection performance or permission to energize a prototype.
