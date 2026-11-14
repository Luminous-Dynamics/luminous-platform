# Evidence-bounded GFM capability claims — v0.1

**Tracking:** [Issue #481 — evidence-bounded GFM claims and test-stage promotion gates](https://github.com/Luminous-Dynamics/luminous-platform/issues/481)  
**GFM domain profile:** [PR #478](https://github.com/Luminous-Dynamics/luminous-platform/pull/478)  
**Model validity:** [PR #480](https://github.com/Luminous-Dynamics/luminous-platform/pull/480)  
**Environment passport:** [PR #453](https://github.com/Luminous-Dynamics/luminous-platform/pull/453)  
**Evidence closure:** [Issue #431](https://github.com/Luminous-Dynamics/luminous-platform/issues/431)

**Status:** synthetic structural claim-support evaluator. It never authorizes claim release and does not authenticate Mycelix receipts.

## 1. Paradigm: no single "validated" ladder

Controller-HIL, power-HIL, laboratory, commissioning and field evidence are not globally substitutable. The claim-support rules are scope-specific, and no claim inherits a broader electrical boundary, physical fraction or temporal population than the actual run observed.

This module checks the bounded tuple: exact configuration, GFM cell/family, electrical boundary, execution mode, run result, component realization map, observables, model binding (when model-conditioned), environment passport, calibration/timebase/witness references and Mycelix receipt references.

## 2. Claim-support scopes

| Claim scope | Minimum evidence boundary | Excluded extrapolation |
|---|---|---|
| Model-conditioned response | Model-only or HIL/P-HIL result with exact model-validity binding for the requested cell/scopes | No physical capability statement |
| Controller implementation | Executed passing run with physical controller under controller-HIL or a more physical stage | No physical converter/plant claim when simulated |
| Physical power interface | Physical controller, PCS and power interface at power-HIL or a more physical stage | No whole-site/lifecycle claim |
| Laboratory device performance | Physical controller, PCS, protection and meter in the exact laboratory configuration | No other sites, firmware or seasons |
| Installed commissioning | Site commissioning record, installed configuration identity and physical controller/PCS/protection/meter | No ongoing reliability/lifecycle claim |
| Bounded field operation | Periodic field record, exact site/configuration, observation window, mission denominator, coverage and requested regimes | No unobserved seasons, sites or fleet-wide reliability |

Converter-terminal evidence cannot satisfy a plant or microgrid POC claim. Physical fractions are checked component by component; a physical controller does not make a simulated inverter or network physical.

## 3. Mycelix ownership and claim release

Each evidence record must carry a Mycelix receipt reference and trust-policy revision. This module does **not** verify either. It only ensures the references are present and returns either:
- unsupported, with deterministic exclusion reasons; or
- structurally supported, pending Mycelix verification and independent technical review.

Even the second disposition has `claim_release_permitted=false`. A later integration must consume an actually verified Mycelix receipt object and independent review outcome. No boolean in a Platform fixture may self-authorize cryptographic verification or claim release.

## 4. Synthetic corpus and validator

- `schemas/gfm-claim-eligibility-v0.1.schema.json`: strict Draft 2020-12 structural schema.
- `schemas/gfm-claim-eligibility.synthetic.json`: six synthetic evidence records, seven claim requests and explicit expected outcomes; one request is an intentionally invalid controller-HIL-to-physical-power claim.
- `tools/measurement-audit/src/gfm_claim_eligibility.rs`: typed evaluator with 18 authored Rust unit tests.

The fixture demonstrates model-conditioned, controller, PHIL interface, laboratory, commissioning and field scopes. All digests, components, run results, sampling windows and receipt references are fictional examples. They are not real tests, real site data or cryptographic receipts.

The evaluator rejects unexecuted/non-passing runs, configuration mismatch, boundary mismatch, missing observables, model-domain mismatch, absent physical roles, inadequate field sampling and missing calibration/witness/receipt references. It never verifies evidence bytes or authenticates the receipt.

## 5. Programme targets

- 100% claims bind exact product/configuration, cell, family, boundary, stage and evidence root.
- Zero claims exceed the physical fraction or electrical boundary present in the run.
- Zero model-conditioned claims outside the exact held-out validation cell and scope.
- 100% physical claims bind instrument calibration, timebase and independent-witness references.
- 100% field claims declare site, installed configuration, sampling window, mission denominator, coverage and observed regimes.
- 100% structurally supported records remain pending Mycelix verification and independent review until those systems return their signed results.
- Every excluded claim returns deterministic reasons; unknown/missing evidence never becomes a pass.

## 6. Claim ceiling

`STRUCTURAL_CLAIM_SUPPORT_ONLY_NOT_RELEASE_AUTHORIZATION_OR_CONFORMITY`

The next stage is the real Mycelix receipt adapter and an independent claim-review workflow. No result here authorizes deployment, switching, certification or public performance claims.
