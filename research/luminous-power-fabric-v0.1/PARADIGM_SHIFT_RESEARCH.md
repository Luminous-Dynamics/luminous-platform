# Paradigm-shift research notes v0.1

**Snapshot:** 2026-11-10  
**Parent:** [Luminous Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Purpose:** translate recent converter and power-device literature into testable architectural hypotheses.  
**Evidence status:** literature-derived hypotheses; none has been implemented or validated by Luminous.

## 1. The important shift: from a converter to a reconfigurable power fabric

Conventional product design often starts from one fixed input/output voltage, one power rating and one converter topology. A more ambitious architecture is a family of interoperable PEBBs whose series/parallel arrangement, control mode and port role can be composed around a declared system configuration.

A 2026 IEEE Access paper reports an experimentally validated 25 kW dual DAB–CLLC bidirectional converter using two modules in parallel on the input side and series/parallel reconfiguration on the battery side. The paper reports a 200–1000 V operating range, 98.37% peak charging efficiency and 97.82% peak discharging efficiency. Its 25 kW EV-charging context is not a matched benchmark for PEBB-LV-01, but it demonstrates a concrete route to wider voltage adaptability without pretending a single fixed-ratio stage is optimal everywhere.

**Luminous hypothesis:** create PEBBs that share electrical and mechanical contracts, then compose or reconfigure them at a higher level. The module contract—not a particular topology—becomes the stable product interface.

Possible long-term advantages:
- One platform can serve several voltage and power families.
- A service team can replace a qualified module rather than a complete converter.
- Different power stages can be optimized for cost, efficiency, isolation or voltage range.
- A system can preserve service during a controlled module isolation event, where the system-level architecture and protection permit it.
- Product qualification evidence can be reused only for unchanged configurations and within a declared scope.

**Non-negotiable constraint:** series/parallel reconfiguration is not a casual runtime feature. It requires a dedicated electrical design, interlocks, contactor/solid-state switch coordination, pre-charge/discharge sequencing, state sensing and a separate fault analysis. Phase 0 does not implement dynamic reconfiguration.

## 2. A semiconductor-neutral platform

A recurring mistake is to equate "modern" with SiC or GaN. Device selection depends on voltage class, current, switching frequency, package parasitics, gate drive, thermal path, system cost and lifetime mission profile.

A 2025 IEEE APEC comparison of low-voltage GaN and silicon devices reported materially lower GaN switching and gate-driver losses in its tested configuration, but also found high dead-time sensitivity and dynamic on-resistance reported at 3–4 times its static value at 1 MHz. These are experiment-specific findings, not universal multipliers. They are enough to justify a rule: **model and measure dynamic behavior; never select a semiconductor by material label alone.**

For the 36–60 V / 500 W envelope, compare:
- Low-voltage silicon MOSFETs as a serious reference, including their total gate-drive and switching losses.
- GaN devices with realistic dynamic RDS(on), dead-time and gate-drive parasitics.
- SiC only where its device/package voltage, thermal and switching characteristics make it sensible; do not assume its high-voltage advantages transfer to a 48 V system.

Each candidate needs a datasheet-backed parameter provenance record and, later, an appropriate double-pulse / converter test plan reviewed by a specialist. The first phase must not infer switching energy from a single headline number.

## 3. Control innovation: optimize the whole operating map

The CLLC literature review from Tsinghua University (2025) identifies complementary research directions: soft-start methods, synchronous rectification, highly dynamic control, bidirectional switching and wide-voltage-gain control. These should become independently testable control variants, not a single bundled "AI optimized" claim.

Potential experiments:
- DAB single-, dual- and extended-phase-shift modulation.
- Light-load circulating-current reduction.
- Adaptive dead-time / synchronous-rectification tuning within hard device limits.
- CLLC frequency-control map across both power directions.
- Safe-start and pre-charge sequences under source and load uncertainty.
- Direction reversal after transfer is ramped to zero.
- Control robustness across component tolerance and temperature.

**Paradigm:** optimize a declared mission-profile objective rather than a single point. Objective terms may include conversion losses, auxiliary power, temperature swing, RMS current stress, acoustic noise, control complexity and service cost. Hard constraints (voltage, current, junction/insulation limits and protection response) are not soft weights and cannot be traded away.

## 4. Protection is part of the power architecture

The ABB SACE Infinitus product illustrates the industry direction: high-speed solid-state interruption combined with electromechanical isolation. This is a reminder that a hybrid design can outperform an all-solid-state design in total conduction loss, isolation, thermal burden and service economics.

Research directions for later products:
- Hybrid current paths with semiconductor interruption and mechanical isolation.
- Protection coordination between module and system.
- DC fault-energy containment and safe discharge.
- Selective isolation of one module without destabilizing all healthy modules.
- Independent local protection that does not depend on communications, AI, cloud services or the supervisory processor.

For each candidate architecture measure conduction loss, interruption behavior, peak let-through current, let-through energy, isolation state, leakage, thermal rise, nuisance trip rate, service procedure and failure mode. A microsecond interruption number alone is not a protection benchmark.

## 5. Repair-first, evidence-bearing electrical equipment

The product should be designed around a serviceable unit boundary:
- Power stage.
- Magnetic component assembly.
- Capacitor/filter assembly.
- Gate-drive/control assembly.
- Sensor assembly.
- Cooling assembly.
- Protection/isolation assembly.

A repair event suspends the claims affected by the repaired parts. A semiconductor or magnetics replacement can affect loss, thermal, EMI, insulation and fault behavior; a firmware change can affect control and protection behavior. The product lifecycle record should therefore trigger scoped requalification rather than silently preserve every prior claim.

Mycelix records provenance and evidence; Sol-Atlas binds the configuration and physical geometry; Symtropy compares model and observed behavior; Symthaea can propose the next experiment. Human engineering review and an independent protection boundary remain essential.

## 6. What should count as a revolution?

Not "AI designed a circuit" and not "peak efficiency is higher by 0.3 points". A meaningful advance would demonstrate one or more of these with matched evidence:

1. Lower lifecycle cost per delivered kWh at equal safety, power quality and availability.
2. Higher full-map efficiency after auxiliary power and cooling are included.
3. Smaller environmental/material footprint at equal qualified continuous output.
4. A real reduction in repair time and discarded material, with post-repair claims requalified.
5. Reduced fault propagation and lower let-through energy under the same source impedance and fault scenario.
6. A reconfigurable module family that reduces SKU count and spares burden without increasing unavailability or safety risk.
7. Reproducible qualification evidence that reduces engineering and certification repetition without overstating evidence reuse.

This is the basis for competing with industry: identify a measurable system-level weakness, define a controlled comparison, show an improvement with equivalent or better constraints, and publish the limitations.

## 7. Consequences for PEBB-LV-01

- Keep DAB and CLLC as the main isolated topology candidates.
- Keep a four-switch non-isolated converter only as a reference architecture.
- Compare silicon MOSFET, GaN and SiC from actual loss data and realistic device models.
- Add modulation/control variants to the experiment matrix, while holding hardware and measurement boundary fixed.
- Do not add series/parallel runtime reconfiguration to the first prototype.
- Design a mechanical/service envelope that can later accept standardized PEBB variants.
- Do not claim 25 kW research-paper performance is a 48 V / 500 W target or direct competitor result.

## 8. New CLLC paradigm: constrain the internal state, not just the gain curve

A particularly useful advance appeared on 30 October 2026 in *IEICE Electronics Express*: Huang et al.,
“Design methodology for a bidirectional CLLC resonant converter based on T-type impedance-network
constraints” (DOI 10.1587/elex.23.20260463).

The paper identifies a gap in conventional design workflows that rely primarily on FHA voltage-gain
curves and inductive-impedance constraints: they may omit the resonant tank's internal operating state
and reactance introduced by secondary-side modulation. It maps the CLLC tank to a T-type impedance
network, represents the secondary active bridge as a complex AC impedance, turns a designated
low-frequency boost state into switching-frequency and resonant-inductance-ratio constraints, and
combines those with FHA gain feasibility. A 2 kW prototype validates forward and reverse ZVS measurements and reports 95.7% maximum forward efficiency (95.2% at rated forward load) and 95.4% reverse efficiency; its 150–300 V input / 75–167 V output, 115 kHz resonance and 75–150 kHz switching are not comparable to PEBB-LV-01.

This suggests a major improvement in our CLLC research process:

1. Use FHA as a fast **gain-feasibility screen**, not as the complete design model.
2. Calculate direction-specific equivalent load impedance / quality factor for charging and
   discharging; do not reuse one Q and claim bidirectional coverage.
3. Map the resonant tank to a T-network or equivalent state model and include secondary modulation
   impedance.
4. Constrain internal resonant state, input impedance angle and commutation energy at every required
   operating point.
5. Verify dead-time and device Coss commutation explicitly; a positive inductive impedance alone is
   not enough if available energy is insufficient.
6. Check gain monotonicity over the selected operating-frequency range and reject a tank that has
   an uncontrolled peak/turning point inside the commanded range.
7. Validate the equation-derived design with time-domain switching simulation and eventually measured
   forward and reverse waveforms.

This changes the optimizer from “find an LC tank with enough gain” to “find a physically admissible
tank whose gain, internal state, impedance, commutation and stress remain feasible across the mission
envelope.”

### Controller timing becomes part of the plant model

A second October 2026 *Electronics* paper by Tao et al. reports a 4.8 kW bidirectional CLLC PV-storage
prototype with 97.45% peak charging efficiency and 96.66% peak discharging efficiency. It combines
FHA tank design, gain monotonicity, inductive input impedance and sufficient magnetizing energy for
Coss commutation with high-resolution PFM using a DSP's HRPWM facilities. Its operating envelope,
power class, turns ratio and cooling are not a match for PEBB-LV-01, but the design lesson is important:
frequency-command quantization, register-update synchronization and dead-time are electrical
performance constraints, not mere firmware implementation details.

For Luminous this means:
- Include achievable switching-frequency resolution and update jitter in the controller specification.
- Bind controller clock tree, PWM resolution, update synchronization and firmware build to every
  model/bench run.
- Quantify gain error caused by frequency quantization at each operating point.
- Check the effect of a frequency update during a current/voltage transient.
- Never let a high-resolution PWM feature substitute for hardware overcurrent / overvoltage / thermal
  interlocks.

### What not to overclaim

The T-type paper's 2 kW forward/reverse ZVS result validates that paper's prototype and design method.
The 4.8 kW PV-storage paper's efficiency results validate its own system and measurement boundary.
Neither proves that our proposed 500 W design is feasible or better. Our immediate software layer
implements only the symmetric FHA gain equation and a discrete gain-curve scan; it does not yet
implement T-network internal-state constraints, impedance-angle calculation, Coss/dead-time energy,
or continuous monotonicity proofs.

## Sources

- IEEE Access, “Design and Experimental Validation of a 25-kW Dual DAB–CLLC Bidirectional DC/DC Converter With an Ultrawide 200–1000 V Operating Range,” published 23 April 2026, DOI [10.1109/ACCESS.2026.3686835](https://doi.org/10.1109/ACCESS.2026.3686835).
- IEEE APEC 2025, “Switching Performance Comparison of Low-Voltage GaN and Si Devices,” DOI [10.1109/APEC48143.2025.10977390](https://doi.org/10.1109/APEC48143.2025.10977390).
- Sun et al., “Review of Analysis Methods and Control Technologies for CLLC Resonant Converters,” Chinese Journal of Electrical Engineering, 11(2), June 2025, DOI [10.23919/CJEE.2025.000131](https://doi.org/10.23919/CJEE.2025.000131).
- ABB, [SACE Infinitus](https://www.abb.com/global/en/areas/electrification/low-voltage/circuit-breakers/solid-state-circuit-breakers/sace-infinitus).
