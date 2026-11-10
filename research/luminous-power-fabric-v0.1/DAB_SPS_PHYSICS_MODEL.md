# DAB SPS physics model and operating-region plan v0.1

**Snapshot:** 2026-11-10  
**Parent:** [Issue #364 — Luminous Power Fabric](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Status:** analytical screening implementation; no hardware correlation or validation result claimed.

## 1. Why this model exists

A DAB converter should not be selected by its ideal power-transfer curve alone. A useful design must
satisfy power demand, RMS/peak current, switching loss, ZVS commutation, thermal limits, control
resolution and component derating simultaneously.

Mahmoud et al. (Engineering Proceedings, 7 September 2026) derive a closed-form lossless SPS
piecewise-current model and compare it with a numerical time-domain solution. They report:
- Closed-form RMS-current discrepancy below 1.3% against their own time-domain model.
- Exact power versus numerical model agreement within approximately 1.5% over their phase sweep.
- FHA power overestimation of 7.9% at 15°, 14.6% at 30°, 20.0% at 45° and 27.3% at 90°.
- A voltage-ratio-dependent SPS switching-current-polarity boundary.
- A one-node loss/thermal screen, with their baseline thermal limit binding near 35° at k=1.

This is a numerical/model-validation paper, not a hardware experimental result. Its 100 V / 28 V
spacecraft case, referred voltage, 20 µH, 50 kHz and lumped 50 mΩ assumptions are not PEBB-LV-01
parameters.

## 2. Rust implementation

The new `dab_sps_physics` module implements the published lossless analytical screen.

The caller supplies:
- Source voltage amplitude.
- Destination voltage referred to the active source side.
- Source-side-referred series inductance.
- Switching frequency.
- Signed phase shift.

Positive phase transfers source-to-destination; negative phase reverses power. For reverse operation,
the caller must re-express the active source/destination voltages and series inductance using a
consistent source-side referral. The helper does not silently swap transformer turns ratio.

### Exact power and maximum power

`k = V2r / V1`

`P_exact = V1 * V2r * phi * (pi - phi) / (2 * pi^2 * fs * L)`

`P_max = V1 * V2r / (8 * fs * L)`

Here `phi` is the positive phase-shift magnitude in the supported `(0, pi/2]` interval.
The sign of power is carried separately by the signed phase input.

### Piecewise current and RMS current

`c = V1 / (2 * pi * fs * L)`

`i0 = c * (-pi * (1-k)/2 - k*phi)`

`delta_i = c * (1+k) * phi`

`i1 = i0 + delta_i`

`Irms^2 = [phi*(i0^2 + i0*delta_i + delta_i^2/3) + (pi-phi)*i1^2] / pi`

The screen reports `i0`, `i1`, ripple, RMS current and peak absolute boundary current. This is the
lossless steady-state piecewise model. The source paper states that its resistive case instead uses a
piecewise exponential solution and a periodicity solver; the lossless formula must not be treated as
exact when the converter's effective series resistance materially changes its current waveform.

### FHA error and applicability

`P_FHA = V1 * V2r * sin(phi) / (2 * pi * fs * L)`

`FHA_error_percent = 100 * (P_FHA - P_exact) / P_exact`

The code reports the error rather than allowing the FHA approximation to masquerade as exact power.
A future controller model should use FHA only inside an explicitly declared error budget; outside
that region, use the exact piecewise model or a switching/time-domain model.

### Analytic ZVS polarity screen

`phi_ZVS_min = (pi/2) * max(0, 1-k)`

The code reports whether the requested phase magnitude exceeds this current-polarity boundary.
This is not a device-level ZVS proof: it does not include MOSFET/SiC/GaN output capacitance,
dead-time, nonlinear Coss, gate resistance, parasitic inductance, commutation energy or individual
switch current polarity in the actual hardware.

### Lumped thermal screen

The optional helper accepts caller-supplied parameters:
- Equivalent conduction-path resistance.
- Hard-switching loss at the exact operating point.
- Residual switching-loss fraction when the analytic ZVS screen is met.
- Ambient temperature.
- Junction-to-ambient thermal resistance.
- Junction-temperature limit.

It calculates `P_cond = Irms^2 * Req`, applies the supplied residual switching-loss factor only if
the analytic polarity screen is met, and estimates a one-node junction temperature. The 5% residual
factor shown in the cited paper is only an illustrative modelling assumption; it must be calibrated
for the selected device and switching conditions. This helper is not an electrothermal device model
or safety function.

## 3. Reproduced literature reference points

For the paper's baseline `V1=100 V`, `V2r=100 V`, `L=20 µH`, `fs=50 kHz`, `k=1`:

| Phase | Exact power | RMS current | FHA overestimation |
|---:|---:|---:|---:|
| 15° | 381.94 W | 4.049 A | 7.85% |
| 30° | 694.44 W | 7.857 A | 14.59% |
| 45° | 937.50 W | 11.411 A | 20.04% |
| 88° | 1,249.38 W | 20.069 A | 27.31% |

These are equation reproductions of the cited paper's analytical baseline, not Luminous prototype
results. The sharp RMS-current growth near maximum transfer is the practical reason not to optimize
only for `P_max`.

For the paper's adopted voltage-ratio polarity screen:
- k=1.00 gives 0° minimum phase.
- k=0.85 gives 13.5°.
- k=0.70 gives 27°.

The analytic polarity screen is necessary only within its declared ideal model; it is not sufficient
to qualify real-device soft switching.

## 4. Design implications for PEBB-LV-01 and later DAB products

1. Use exact piecewise SPS current/power as an independent oracle for simple SPS test fixtures.
2. Quantify FHA error over the whole phase / voltage-ratio operating map.
3. Select turns ratio and series inductance to optimize ZVS coverage, rating utilization, reactive
   power and current stress together—not only peak efficiency.
4. Add EPS/TPS only after common waveforms, phase sign conventions and fair baselines are reviewed.
5. Tie all current limits, loss terms and thermal assumptions to a specific component, BOM and
   operating point.
6. For a real device ZVS determination, calculate commutation charge/energy against Coss and dead
   time and then validate with measured switching waveforms.
7. Keep thermal, efficiency, current, power density, isolation and cost as separate Pareto dimensions.

The 2026 multidimensional DAB paper by Ren et al. is an important next reference: its 1 kW prototype
uses multiobjective particle-swarm optimization plus TPS to balance ZVS range, rating utilization
and reactive power, with peak efficiency above 95.9%. The next Luminous DAB optimizer should optimize
this vector rather than simply maximizing efficiency.

## 5. Explicit limitations and next work

- No lossy periodicity solver is implemented.
- No EPS/TPS current equations are implemented.
- No actual switch-by-switch ZVS/Coss/dead-time energy test is implemented.
- No real magnetic, semiconductor, connector, PCB or transformer model is included.
- No model correlation with hardware has occurred.
- No thermal camera, power analyzer or oscilloscope data is present.

Next work: implement the resistive piecewise exponential DAB periodicity solver from the cited paper,
compare it against the lossless closed form as Req→0, then add a separate multiobjective design-space
sweep of turns ratio, fs*L, modulation and voltage/load mission profile. Add measured-waveform
correlation only after a reviewed current-limited low-voltage prototype and qualified test procedure.
