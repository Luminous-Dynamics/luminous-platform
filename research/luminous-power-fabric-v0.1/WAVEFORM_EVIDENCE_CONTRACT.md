# Waveform and power-quality evidence contract v0.1

**Parent:** [Issue #438](https://github.com/Luminous-Dynamics/luminous-platform/issues/438)  
**Stack:** [Controller trace PR #436](https://github.com/Luminous-Dynamics/luminous-platform/pull/436)  
**Status:** versioned capture schema and synthetic metadata fixtures only. No raw waveform DSP, PQ algorithm execution, Rust test execution or physical measurement is claimed.

## Standards snapshot

- [IEC 61000-4-30:2025, corrected version 2026-07](https://webstore.iec.ch/en/publication/71611), Edition 4.0: in-situ PQ methods for 50/60 Hz AC; Class A and Class S. Covers frequency, voltage, flicker, dips/swells, interruptions, transients, unbalance, harmonics/interharmonics, RVC and current parameters; Annex C/D includes 2–150 kHz conducted emissions.
- [IEC 61000-4-7:2002+AMD1:2008](https://webstore.iec.ch/en/publication/4228): harmonic/interharmonic measurement instrumentation through 9 kHz; Edition 3 is under development.
- [IEEE 519-2022](https://standards.ieee.org/ieee/519/10677/): steady-state harmonic goals at the PCC; P519 revision is separate from transient-event acceptance.
- [IEEE 1547-2018](https://standards.ieee.org/ieee/1547/5915/) and [IEEE 1547.1-2020](https://sagroups.ieee.org/scc21/standards/standards-1547-1/): DER interconnection/interoperability and test procedures. Product/jurisdiction applicability is specific; IEEE 1547-2018 is written considering 60 Hz, so 50 Hz South African applicability needs independent SANS/utility determination.

IEC 61000-4-30 is an in-situ PQ method, not a substitute for high-speed inrush/switching capture. CT/PT/probe transfer functions, phase error, bandwidth, saturation and channel skew need separate evidence.

## Evidence planes

| Plane | Measures |
|---|---|
| Customer service | process/motor outcome, critical continuity, unserved service |
| In-situ PQ | frequency, voltage, dips/swells/interruption, RVC, flicker, unbalance, harmonics |
| Harmonic spectrum | voltage/current harmonics and interharmonics |
| High-speed transient | pre/post-trigger voltage/current around start, step, transfer, island and protection |
| Protection | trip/no-trip, interruption, isolation and let-through/I²t |
| Energy | real input/output, auxiliaries and storage delta at matched POCs |

## Capture passport

The schema binds exact hardware/wiring/protocol, event ID, channel measurand and plane, instrument/transducer, transfer-function digest, calibration scope/status/expiry, timebase/clock epoch, channel skew, sample rate/count, analog bandwidth, anti-alias cutoff/filter, trigger/pre/post window, sample-quality counters, raw waveform digest and precomputed metric method/window/channel map/uncertainty/evidence state.

The record classes distinguish Class A, Class S and high-speed transient capture. These labels are not interchangeable. A synthetic record labelled Class A is not an IEC-compliant instrument or measurement.

Six synthetic examples cover valid high-speed capture, Class A, Class S, channel-skew failure, clipped capture and expired calibration. JSON parsing and authoring-time consistency checks are not full Draft 2020-12 validation. This contract does not process waveform samples, calculate RMS/FFT/THD/flicker, authenticate Mycelix receipts, evaluate protection let-through or establish conformity.

**Claim ceiling:** synthetic metadata only. No physical measurement, HIL, protection test or conformity result is claimed. No energization, fault injection or grid connection is authorized.
