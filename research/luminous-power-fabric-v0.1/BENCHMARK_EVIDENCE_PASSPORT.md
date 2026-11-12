# Replayable benchmark evidence passport and matched-comparison gate

**Snapshot:** 12 November 2026  
**Tracking:** [#429 — replayable benchmark evidence passport](https://github.com/Luminous-Dynamics/luminous-platform/issues/429)  
**Stack:** [mission energy #423](https://github.com/Luminous-Dynamics/luminous-platform/pull/423) → [power-duration events #426](https://github.com/Luminous-Dynamics/luminous-platform/pull/426)  
**Evidence status:** schema, typed comparator and synthetic test fixtures only. No physical benchmark result or superiority claim is made.

## 1. Paradigm shift: a benchmark result must carry its proof context

A scalar such as "97% round-trip efficiency" is not a reproducible benchmark. It is uninterpretable without the energy boundary, auxiliary feeder treatment, SOC window, duty cycle, environment, source/fuel conditions, calibration, sample selection, processing policy and exact equipment revision.

Power Fabric therefore treats the **benchmark run and its evidence bundle** as the primary record. A result is a reference to:
- the exact system configuration and test method;
- the primary point of connection and any separate auxiliary POC;
- the matched useful-service, event, environment, source/fuel, initial storage and reserve policies;
- raw-data digest, timebase and sample-selection/processing lineage;
- instruments, calibration scope/method, expiry and evidence references;
- metric values with units, measurement method, uncertainty and provenance;
- operational fault, maintenance and dispatch logs where relevant;
- evidence class, data rights and explicit claim ceiling.

The canonical benchmark pair has two distinct product configuration digests. They are expected to differ. The comparison gate instead insists that the scenario and measurement conditions match.

## 2. Matched-comparison rules

The typed comparator in tools/measurement-audit/src/benchmark_comparison.rs checks:
- exact test protocol, primary/auxiliary POC, service boundary and load/service profile;
- event schedule, environment and source/fuel conditions;
- initial storage condition and final reserve policy;
- data-treatment, sample-selection, preprocessing and timebase method;
- mission duration, reporting window and sample-completeness policy;
- metric identity/unit, measurement method and measurand scope;
- current timebase, current calibration, calibration expiry, calibration scope and calibration method;
- raw-data digest for physical runs and evidence references for every metric.

It retains the complete reference and candidate run snapshots alongside the evidence-bundle/raw-data/history digests, so all match keys, calibration references, POC identities, run classifications and data rights remain attached to the report. Metric deltas are candidate minus reference, and the first-order combined uncertainty is the conservative sum of absolute uncertainty bounds. The comparator does not assume independence or use an RSS calculation.

A structural match is separate from physical-measurement eligibility. A synthetic pair may match every scenario key and still be ineligible as physical evidence. A physical measurement pair can be comparison-eligible without being independently witnessed. The stronger independent-claim evidence eligibility additionally requires both evidence classes to be independently witnessed/accredited and an independent-assessment reference for each run.

This output is only an **eligibility gate**. It does not decide which metric is better, test statistical significance, calculate a superiority margin, verify cryptographic authenticity, validate calibration certificates, or establish standards conformity. A SOTA claim still needs pre-registered metrics/effect sizes, repeated runs, a declared uncertainty/guard-band method, failed-run disclosure, independent review and physical evidence.

## 3. Laboratory tests versus site-operational assessment

The evidence class and run mode are both explicit and must be consistent:
- vendor assertion → vendor-published specification;
- modelled → simulation;
- synthetic → synthetic;
- measured / independently witnessed / accredited → controlled laboratory, production acceptance, site-operational assessment or field mission.

The run mode distinguishes:
- controlled laboratory;
- production acceptance;
- site-operational assessment;
- field mission;
- vendor-published specification;
- simulation;
- synthetic.

This distinction matters. IEC TS 62933-2-3:2025 covers EES operational performance assessment after commissioning. Its public preview describes representative periodic sampling, preprocessing of missing/anomalous/noisy data, and use of operational data and fault/maintenance/dispatch records. It identifies relevant operational performance areas including actual capacity, input/output power, round-trip efficiency, auxiliary consumption, available hours, equivalent operation factor and availability. The public preview describes the site assessment as a relative quantitative assessment under operational and environmental boundary conditions, not a precise laboratory measurement. Use the precise laboratory test methods in the relevant IEC 62933-2-1 scope when accuracy is required.

The passport implements a stronger, explicit record:
- site-operational runs require a declared minimum of at least three representative sample events;
- fault and maintenance log digests must be present; dispatch log digest is retained where available;
- sample-selection, time-series preprocessing and missing-data acceptance policies are versioned;
- all raw operational data and histories are retained by digest.

The three-sample rule is aligned with the public preview's general recommendation, but the comparator does not claim to implement or certify IEC conformance. Detailed requirements must be confirmed against the licensed standard and the actual product's applicability.

## 4. Point of connection and auxiliary power

The primary POC must identify the useful-service measurement plane. If auxiliary systems are supplied from a separate feeder, that auxiliary POC must be recorded separately. This prevents cooling, controls or HVAC from disappearing outside the energy boundary and making a system appear more efficient.

For an EES comparison, report as applicable:
- actual discharged energy at the declared POC;
- charge/discharge input/output power;
- round-trip and duty-cycle efficiency;
- auxiliary power/energy, including separately fed auxiliaries;
- available hours, equivalent operation factor and availability;
- planned/unplanned outage hours/factors;
- source and storage state, operating/ambient conditions, and fault/maintenance/dispatch history.

Do not collapse vendor nominal capacity, battery-side DC energy and net usable POC energy. A 1,200 kWh nominal battery does not prove 1,200 kWh deliverable at the customer POC after DoD, conversion, auxiliaries and operating constraints.

## 5. Industry comparison: what the public specifications do and do not say

Atlas Copco's published ZBC 1000-1200 data lists 1,200 kWh nominal capacity, 800 kW continuous mode, a 90% recommended depth of discharge, 70% EOL, and an expected 6,000 cycles under its specified 25°C / DoD / EOL conditions. Its separate product configurations vary by power, capacity and discharge C-rate.

Those figures are a useful vendor baseline, not an independent field result. A fair long-term comparison should add POC output energy, auxiliary consumption, available hours, outage factors, actual maintenance/fault history and environmental operating conditions. Do not convert the headline 6,000-cycle statement directly into a universal calendar life or an unconditional warranty.

The first Power Fabric competitive claim should be **better measured service per lifecycle input**, not a higher datasheet number:
1. equal POC/useful-service boundary;
2. equal mission and reserve obligation;
3. exact test and data-treatment conditions;
4. calibrated metrology with compatible uncertainty;
5. repeated, representative data and operational histories;
6. independent review and all failed/omitted data disclosed.

## 6. Cross-project ownership

| Owner | Responsibility |
|---|---|
| Symtropy | Source/plant model, generator/inverter replay, fuel and interval/event traces |
| Mycelix | Instrument identity, calibration scope/method, timebase, uncertainty, evidence invalidation and raw-sample provenance |
| Sol-Atlas | Site/environment, spatial topology, deployment configuration and operating context |
| Symthaea | Adversarial benchmark-case generation and experiment selection; no evidence authority |
| Luminous Platform | Canonical passport, pairwise comparability gate, energy/event evidence composition and claim ceiling |

The comparator only checks declared evidence status and digest shape. It does not authenticate certificates or hash files itself, and it does not create a second metrology registry. The cryptographic closure, signature/revocation and deterministic replay interface is deliberately deferred to [issue #431](https://github.com/Luminous-Dynamics/luminous-platform/issues/431), with Mycelix as owner.

## 7. Implementation and synthetic fixture

- Rust module: tools/measurement-audit/src/benchmark_comparison.rs.
- JSON contract: schemas/benchmark-evidence-pair-v0.1.schema.json.
- Matching synthetic pair: schemas/benchmark-evidence-pair.synthetic.json.
- Adversarial mismatched-service pair: schemas/benchmark-evidence-mismatch.synthetic.json.

The matching fixture deliberately uses synthetic evidence class. It demonstrates that both records can match structurally while remaining ineligible for physical comparison. Its 100 Wh reported difference is a fake arithmetic value used to exercise the gate—not an efficiency or product result.

The comparator is dependency-free and does not add JSON parsing/serialization. The schema is the external data contract; typed Rust input is the current library API. No claim of full Draft 2020-12 schema validation, Rust compilation or test execution is made until an appropriate validator/toolchain is run.

## 8. Standards references

- [IEC TS 62933-2-3:2025 — site-operational EES performance assessment](https://webstore.iec.ch/en/publication/77334).
- [IEC TS 62933-2-2:2022 — application duty cycles and performance testing](https://webstore.iec.ch/en/publication/64570).
- [IEEE 2030.3-2016 — EES test procedures](https://standards.ieee.org/ieee/2030.3/5076/).
- [ISO 8528-5:2025 — RIC-engine / AC-generator set performance](https://www.iso.org/standard/85960.html).

The standards inform the benchmark design; they do not confer product certification or automatic South African SANS/NRCS approval.