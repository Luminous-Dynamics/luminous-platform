# GFM tested capability envelope — v0.1

**Tracking:** [Issue #477 — tested GFM capability envelope, weak-grid stability and multi-vendor qualification](https://github.com/Luminous-Dynamics/luminous-platform/issues/477)  
**Controller/HIL matrix:** [PR #446](https://github.com/Luminous-Dynamics/luminous-platform/pull/446)  
**Operating-mode graph:** [PR #455](https://github.com/Luminous-Dynamics/luminous-platform/pull/455)

**Status:** synthetic test-plan profile and typed read-only domain-coverage validator. No GFM device capability, model validity, grid-code compliance, field performance or SOTA result is claimed.

## 1. Research baseline (snapshot 2026-11-14)

- **UNIFI GFM Specifications v3** was released on 2026-01-30. It is vendor-neutral, defines inverter- and system-level requirements, adds frequency-domain criteria and quantitative pass/fail tests, and is the basis for IEEE P2800.1: https://unificonsortium.org/unifi-version-3-gfm-specifications-released/ and https://research-hub.nlr.gov/en/publications/unifi-specifications-for-grid-forming-inverter-based-resources-ve-2/
- **IEEE P2800.1** is an active IEEE Recommended Practice project, approved 2025-12-10, for GFM IBR/CBR functional capabilities/performance and model-based tests. It is not a published standard yet: https://standards.ieee.org/ieee/2800.1/12387/
- A review published 2026-10-19 synthesizes 16 industry-oriented GFM tests and highlights model fidelity, finite current/energy, scalability, multi-vendor interaction and harmonisation gaps: https://www.mdpi.com/1996-1073/19/20/4966
- IEA PVPS Task 13 (2026-07) emphasizes measured capacity, power tolerance, internal resistance, RTE, response time and standby losses; field outcomes depend on topology, inverter, controls, duty, temperature and firmware: https://iea-pvps.org/key-topics/assessing-reliability-battery-systems-plants-2026/

This profile supplements, but does not replace, UNIFI v3, applicable grid codes, IEEE 2800/2800.2 or licensed normative requirements.

## 2. Paradigm: a capability is a domain, not a badge

A GFM claim must be tied to exact product, firmware, controller, protection and topology configurations; electrical measurement boundary; P/Q operating point; SOC; DC energy and current headroom; thermal state; SCR and X/R; and the exact disturbance plus observed response.

A single nominal pass cannot be generalized to all operating conditions. v0.1 represents the declared operating domain as a finite set of explicit cells. It intentionally forbids interpolation and extrapolation outside those cells. A future continuous response surface would require a separately validated model, uncertainty and out-of-domain detector.

## 3. Seven added GFM-specific test families

| ID | Family | Required evidence focus |
|---|---|---|
| GFM-01 | Operating-point/headroom envelope | P/Q, SOC, energy, current and thermal margin |
| GFM-02 | Weak-grid SCR/X/R sweep | POC grid strength, response, damping and minimum stable SCR |
| GFM-03 | Phase-angle jump | P transient, peak current, synchronism and recovery |
| GFM-04 | Current-limit/fault ride-through | Voltage-time profile, peak/sustained current, P/Q priority, DC energy and recovery |
| GFM-05 | Negative-sequence/unbalanced fault | Negative-sequence current, phase imbalance, DC ripple and recovery |
| GFM-06 | Frequency-domain impedance scan | Impedance evidence, resonance, gain/phase margins and model/measurement deviation |
| GFM-07 | Multi-vendor interaction | P/Q sharing, circulating power, oscillation modes/damping and stability margin |

The family-specific observables are mandatory in the typed validator. Acceptance thresholds are **not** invented here: every test references a versioned UNIFI/grid-code/product criterion bundle supplied by the responsible technical owner.

The profile distinguishes converter terminals, plant POC and microgrid POC. Evidence at one boundary cannot be silently promoted to another.

## 4. Coverage contract

Machine-readable artifacts:
- `schemas/gfm-capability-envelope-v0.1.schema.json`: strict Draft 2020-12 structural schema.
- `schemas/gfm-capability-envelope.synthetic.json`: three synthetic operating cells, seven test families and five capability requests.
- `tools/measurement-audit/src/gfm_capability_envelope.rs`: typed profile validator with 16 authored Rust tests.

Every requested capability maps to exact operating cells, test families and electrical boundary. A planned test must bind disturbance and criterion references, execution modes, mandatory observables, and positive/negative/recovery controls. Current-limit and unbalanced-fault tests require a planned power-HIL or physical-laboratory mode; frequency-domain tests require EMT or physical-laboratory mode; multi-vendor tests require at least two distinct participant configuration digests.

The output describes only how completely a **test plan** covers its requested domain. It does not say that any test was executed or passed.

## 5. Competitive benchmark targets

Research targets, not achieved results or universal grid-code thresholds:

- 100% of declared capability requests covered by explicit operating-cell × test-family × boundary combinations.
- 100% cells bind product/topology/controller/protection digest, P/Q, SOC, current/energy headroom, temperature, SCR, X/R, pre-voltage and pre-frequency.
- 100% tests bind a versioned acceptance-criteria reference and test boundary.
- 100% required observables are present for the selected test family.
- 100% fault/current-limit tests retain peak and sustained current, current priority, energy consumption, synchronism and recovery.
- 100% weak-grid claims identify tested SCR/X/R range and relevant frequency-domain/stability evidence.
- 100% multi-vendor claims include distinct vendor/control configuration identities under matched system conditions.
- Zero claim extrapolation outside tested cells.
- Report CHIL/PHIL/model-to-hardware deviation by observable and timescale with preregistered tolerances from the applicable criterion owner.
- No average score can compensate for instability, protection trip or loss of synchronism.

## 6. Validation and claim ceiling

The JSON schema and synthetic fixture are parsed in the authoring environment; authoring-time checks confirm unique identities, valid digests, valid cell references and complete capability-request coverage. The Rust tests are authored but not executed. No Rust compiler/Cargo, formatting or Clippy result is claimed.

Claim ceiling: `SYNTHETIC_GFM_TEST_PLAN_AND_DOMAIN_COVERAGE_ONLY_NOT_CAPABILITY_OR_CONFORMITY`.

The next stage is to map the seven test families against the full licensed UNIFI v3/IEEE P2800.1 criteria, then execute an exact product profile in EMT/CHIL/PHIL/lab with an independent witness and calibrated instrumentation.
