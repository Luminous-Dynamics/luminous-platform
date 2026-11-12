# Power Fabric industrial mission-profile corpus — v0.1

**Tracking:** [#449 — rights-aware industrial mission-profile corpus and cross-application holdouts](https://github.com/Luminous-Dynamics/luminous-platform/issues/449)  
**Canonical contract:** `schemas/field-mission-profile-v0.1.schema.json` (reuse; no new profile ontology)

**Status:** six synthetic example profiles only (one existing construction profile plus five new profiles). No measured customer/site duty cycle or physical performance is claimed.

## 1. Why a portfolio matters

A fair power system or dispatch-controller benchmark needs a workload population. The existing construction-shift fixture is a useful smoke test, but it cannot represent the duty of a water station, mine, port, plant shutdown or isolated communications site.

The corpus provides separate mission profiles that can be matched across generator-only, BESS-only, generator+BESS and future Luminous-integrated candidates. It is a workload contract portfolio, not a physics model and not a performance result.

## 2. Corpus index

The machine-readable index is [mission-profile-corpus-v0.1.json](./schemas/mission-profile-corpus-v0.1.json), governed by [mission-profile-corpus-v0.1.schema.json](./schemas/mission-profile-corpus-v0.1.schema.json). Each profile reuses the canonical field mission profile schema.

| Profile | Application family | Workload stressors | Evidence |
|---|---|---|---|
| PF-FIELD-CONSTRUCTION-SYNTHETIC-001 | Construction | Pump start, intermittent tools, lighting, controls | Synthetic |
| PF-WATER-PUMP-SYN-001 | Plant auxiliary | Critical pumping, staged starts, source loss/return | Synthetic |
| PF-MINING-DEWATER-VENT-SYN-001 | Remote operations | Dewatering, ventilation, compressor, heat/dust/altitude | Synthetic |
| PF-PORT-CRANE-SYN-001 | Other | Crane/hoist cycles, reefer loads, transients, source transfer | Synthetic |
| PF-INDUSTRIAL-SHUTDOWN-SYN-001 | Industrial maintenance | Compressor, welding, pumps, lighting, temporary supply | Synthetic |
| PF-REMOTE-COMMS-WATER-SYN-001 | Remote operations | Critical auxiliaries, endurance, PV/BESS/generator | Synthetic |

The index suggests holdout groups by application family. A confirmatory split must be preregistered and must not split correlated segments from one trace across training/test.

## 3. Rights and evidence policy

All v0.1 profiles are authored synthetic assumptions under CC0-1.0. No customer, operator, vendor or site telemetry is embedded.

Future measured profiles must not be added until the owner grants the stated rights. Their records must bind raw-data digest, source channel map, units, sampling interval, timebase, instrumentation/calibration references, missingness and transformations. A modelled reconstruction remains modelled; synthetic augmentation cannot increase the count of measured independent samples.

Use [issue #434](https://github.com/Luminous-Dynamics/luminous-platform/issues/434) for claim-specific sample sufficiency, cluster dependence and representativeness. Many samples from one operating episode do not automatically support a multi-site reliability claim.

## 4. How to use the corpus

1. Choose one application profile and freeze its exact digest and acceptance requirements.
2. Bind candidate and reference to the same load/criticality schedule, source availability, site/environment and event profile.
3. Apply the same initial storage/source state, information budget and terminal reserve obligation.
4. Score customer-POC service, local critical events, energy/fuel, reserve, recovery and power quality separately.
5. Repeat across a preregistered suite of profiles and keep whole application families/sites/seasons held out.
6. Preserve failures and indeterminate outcomes; do not average away a failed critical service.

A profile specifies demand and context. It does not supply a validated pump, generator, battery, inverter, thermal, grid or protection model. Those remain with their respective owners.

## 5. Current limitations and next evidence

- All profile values are synthetic and illustrative.
- No profile has raw field telemetry, calibrated meters, customer approval or independent witness evidence.
- The present schema does not contain detailed waveform files or electro-mechanical load physics.
- Port/crane regeneration is not represented; crane loads are positive consumption only.
- Hazardous-area and site electrical requirements are not qualified.
- Actual acceptance limits must be supplied by the customer/product classification and applicable engineering standards.

The next promotion step is to obtain one rights-cleared, instrumented mission profile from a consenting pilot site, validate timebase/metrology and preserve raw data under the Mycelix evidence-closure boundary. Do not describe the synthetic corpus as an industrial benchmark result until matched physical runs exist.
