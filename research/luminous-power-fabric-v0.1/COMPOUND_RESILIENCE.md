# Matched compound cyber-physical GFM resilience — v0.1

**Tracking:** [Issue #482](https://github.com/Luminous-Dynamics/luminous-platform/issues/482)  
**GFM domain:** [PR #478](https://github.com/Luminous-Dynamics/luminous-platform/pull/478)  
**Held-out model validity:** [PR #480](https://github.com/Luminous-Dynamics/luminous-platform/pull/480)  
**Transition graph:** [PR #455](https://github.com/Luminous-Dynamics/luminous-platform/pull/455)  
**Command-intent security contract:** [PR #460](https://github.com/Luminous-Dynamics/luminous-platform/pull/460)

**Status:** typed offline factorial benchmark evaluator. It does not run a controller, inject attacks, operate switchgear or qualify a physical device.

## 1. Research position

The current GFM qualification stack contains a finite operating domain, capability test matrix and held-out time/frequency model-validity profile. Those are necessary but not sufficient for compound resilience. A controller may pass a weak-grid disturbance test and a false-data test separately, then fail when both occur together.

A January 2026 CHIL paper compares droop, VSG and dVOC on an OPAL-RT/EXata microgrid testbed and validates a TI C2000 model-reference layer against reference and measurement FDI. It reports VSG active-power settling at 0.63 s versus 2.14 s for droop and 1.835 s for dVOC in its particular setup. This is an important comparator, not a universal winner or full cross-vendor compound-resilience benchmark: https://doi.org/10.3390/en19030710

UNIFI GFM Specifications v3 (2026-01-30) add quantitative and frequency-domain criteria; IEEE P2800.1 is an active standardization project. IEEE 2800.2-2026 was published 2026-06-30 for plant-level conformity-assessment procedures. IEC TS 62898-3-5:2026 covers microgrid controller HIL, commissioning and periodic tests. This programme is an additional defensive experimental design, not a substitute for those standards.

## 2. Paradigm: matched 2×2 factorial design

Every frozen operating cell and scenario seed uses four arms:
- B: baseline, neither physical disturbance nor cyber manipulation;
- P: physical disturbance only;
- C: cyber manipulation only;
- PC: the same physical disturbance and cyber manipulation together.

For every adverse metric, calculate the interaction:

I = Y(PC) − Y(P) − Y(C) + Y(B)

For higher-is-worse metrics, positive interaction denotes excess compound harm beyond the sum of isolated responses. The evaluator reports all four outcomes and each interaction term; it never creates a scalar resilience score. This is a scenario-level contrast, not a population causal estimate.

## 3. Typed binding contract

The evaluator consumes the existing GFM capability profile, held-out model-validity profile and operating-mode transition graph. It requires exact GFM profile/cell/test identity; matching disturbance family and immutable physical/cyber profile references/digests; and explicit event offsets, with P/PC and C/PC arm offsets matched.

It binds product hardware, firmware, controller, protection, topology, model and analysis build; held-out model case with a separate calibration campaign and dataset root; physical reference mode; model P/Q, SOC, temperature, SCR and X/R to the GFM operating cell; and exact scenario seed, site, initial state/epoch/SOC, timebase, observation window, acceptance criteria, transition path and execution mode across all arms. The declared transition path must use permitted edges and reach the observed post-state.

The schema contains defensive factor descriptors, not live-control procedures, attack payloads or credentials.

## 4. Non-compensatory gates

Every arm is evaluated independently against owner-approved thresholds:
- unserved critical Wh and terminal reserve;
- maximum voltage/frequency deviation and RoCoF;
- recovery duration and local control deadline misses;
- correct protection action, zero forbidden transitions and zero unknown-state mutations;
- cyber arms: detection and successful mitigation within declared latency;
- no-attack arms: false-positive alert allowance.

The compound scenario passes only if every arm passes and each interaction term stays within its preregistered positive-harm limit. Good average performance cannot offset a critical-service or protection failure.

Physical-evidence eligibility is separate. A synthetic/modelled factorial block may demonstrate the arithmetic and structural logic, but it cannot make a physical claim. All arms need current raw-data, calibration, timebase, witness and result references from an allowed measured execution mode before physical-evidence eligibility can be true.

## 5. Synthetic oracle

One synthetic weak-grid plus PCC-voltage-FDI block is supplied. Interaction terms:

| Metric | Interaction |
|---|---:|
| Unserved critical energy | +15 Wh |
| Voltage deviation | +0.11 pu |
| Frequency deviation | +0.051 Hz |
| RoCoF | +0.52 Hz/s |
| Recovery duration | +300 ms |
| Terminal reserve deficit | 0 Wh |

All inputs are synthetic; this structural result is not physical performance evidence.

## 6. Benchmark targets

- 100% matched B/P/C/PC blocks for every claimed operating cell × disturbance × attack family.
- Zero mismatch in configuration, state/SOC, seed, timebase, observation window, profile roots, event offsets and acceptance criteria.
- Zero critical-service, protection, forbidden-transition or unknown-state mutation failures within a claimed compound envelope.
- Detection/mitigation latency within owner-approved local deadlines.
- Publish arm outcomes, interaction contrasts, false positives/negatives and denominators separately.
- Repeat across independent site/mission/event blocks and report uncertainty before generalization.
- No interpolation outside the tested GFM/model domain.

## 7. Claim ceiling

The module checks structure, matched contexts, declared outcomes and factorial arithmetic. It does not authenticate raw evidence, execute cyber attacks, validate controller behaviour, prove causal independence, establish HIL/PHIL or site performance, demonstrate conformity or prove SOTA superiority.

Claim ceiling: `SYNTHETIC_OR_STRUCTURAL_COMPOUND_RESILIENCE_QUALIFICATION_ONLY_NOT_HIL_OR_FIELD_CLAIM`.
