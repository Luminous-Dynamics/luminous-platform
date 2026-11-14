# Model validity envelope and held-out validation — v0.1

**Tracking:** [Issue #479 — model validity envelope and held-out physical validation](https://github.com/Luminous-Dynamics/luminous-platform/issues/479)  
**GFM domain profile:** [PR #478](https://github.com/Luminous-Dynamics/luminous-platform/pull/478)  
**Environment passport:** [PR #453](https://github.com/Luminous-Dynamics/luminous-platform/pull/453)  
**Plant conformity context:** IEEE 2800.2-2026

**Status:** synthetic model-validation protocol and typed profile-coverage validator. No model or physical reference is validated by this artifact.

## 1. Why model identity and validation domain matter

The model and hardware are not interchangeable artifacts. The profile binds model binary/parameter set, product hardware, firmware, controller, protection, topology and analysis build. A model that does not match the installed configuration cannot support a claim about that installation.

The profile separates calibration/tuning data from held-out validation data. The two datasets must have different content roots and campaign identities. Reusing tuning observations as independent validation is rejected.

## 2. Time-domain and frequency-domain validation are separate

The profile treats these as distinct scopes:

- **Time domain:** voltage/current/P/Q/frequency/phase response across the declared disturbances and observation windows.
- **Frequency domain:** impedance magnitude/phase, resonance frequency and gain/phase margins over a declared scan range.

A 2025 IEEE hardware campaign on a 2 MVA PV inverter reports that time-domain transient tests could miss critical resonance modes that frequency-domain scans exposed: https://ieeexplore.ieee.org/abstract/document/11225174

Frequency-domain claims in this v0.1 profile require EMT switching or EMT average fidelity and a planned power-HIL or physical-laboratory reference. This is a structural test-plan gate, not a declaration that the model passes.

## 3. Held-out physical reference contract

Each validation case binds:
- exact model and hardware configuration identity;
- GFM operating-cell identity and source profile reference;
- held-out validation dataset root (not the calibration root);
- raw data digest, processing build digest and timebase reference;
- instrument calibration receipt and independent witness references;
- physical reference mode (power-HIL, physical lab, commissioning or field event);
- test-specific acceptance-criteria revision;
- required observable IDs and residual records.

Residual records retain normalized RMSE, peak absolute error, measurement uncertainty and timing lag, plus model/measurement/residual summary and uncertainty references. Metric kinds are tied to observables: resonance-frequency shift, gain-margin difference and phase-margin difference use distinct residual types. Owner criteria define acceptable errors; this validator does not invent universal tolerances or decide pass/fail.

## 4. Claim-domain rule

Claims are limited to exact held-out cells and scopes. A request for time- and frequency-domain validation at a cell requires both validation cases at that same cell. No model result is extrapolated to untested P/Q, SOC, temperature, SCR, X/R, topology, firmware or protection configurations.

A firmware, hardware, controller parameter, topology, protection, model parameter or relevant calibration change triggers impact analysis and targeted revalidation. A new model hash does not inherit old validation.

## 5. Synthetic fixture and validator

Files:
- `schemas/model-validity-envelope-v0.1.schema.json`: strict Draft 2020-12 structure.
- `schemas/model-validity-envelope.synthetic.json`: synthetic model identity, separate calibration/held-out datasets, time/frequency validation cases and residual records.
- `tools/measurement-audit/src/model_validity_envelope.rs`: typed read-only validator with 15 authored Rust tests.

The report is always `STRUCTURALLY_COMPLETE_TEST_PLAN_ONLY` and `model_validation_claim_eligible=false`. It does not authenticate evidence, prove data independence, evaluate acceptance thresholds, measure model error or set a physically validated status.

## 6. Industry position and targets

IEEE 2800.2-2026 provides the plant-level conformity assessment context for applicable bulk-system IBRs. NERC EMT modelling guidance recommends matching model versions to firmware and validating against factory, HIL, field or lab evidence. VDE-FNN GFM guidance v2.1 (April 2026) includes measurement-based validation for EMT and RMS models. These sources reinforce that model fidelity and configuration linkage are separate obligations from controller-HIL success.

Research targets, not results:
- 100% claimed cells have held-out time/frequency validation at each required scope.
- Calibration and validation roots/campaigns are disjoint.
- 100% records bind model/device/firmware/controller/protection/topology identities.
- 100% residual metrics carry units, measurement uncertainty, time/frequency window and criterion revision.
- No frequency-domain claim passes based on transient-only evidence.
- Report per-regime residual vectors and worst-case error, not only global mean fit.
- Zero extrapolation beyond validated cells.

## 7. Claim ceiling

`SYNTHETIC_MODEL_VALIDITY_PROFILE_ONLY_NOT_MODEL_VALIDATION_OR_CONFORMITY`

The next step is to bind these references to Mycelix evidence receipts/calibration interfaces and execute matched model/measurement campaigns using qualified hardware, calibrated instrumentation and independently reviewed acceptance limits.
