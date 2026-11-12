# Measured mission-service envelope

**Snapshot:** 2026-11-12  
**Status:** deterministic offline audit proposal; the fixture is synthetic  
**Owner:** [#420](https://github.com/Luminous-Dynamics/luminous-platform/issues/420)  
**Parent architecture:** [Power Fabric PR #414](https://github.com/Luminous-Dynamics/luminous-platform/pull/414)  
**Matched-service benchmark:** [#419](https://github.com/Luminous-Dynamics/luminous-platform/issues/419)

## 1. Paradigm shift: qualify useful service, not a nameplate

A power system is not useful merely because it has a high kW rating, a large kWh label, a fast semiconductor switch or a vendor-stated cycle count. It is useful when it delivers a declared service under a defined mission profile, with acceptable interruptions and power quality, and still meets its terminal-state obligations.

The comparison unit is therefore a **complete, measured mission episode**:

- Same requested load/service profile and duration.
- Same start-state storage energy and declared operating conditions.
- Same service boundary, electrical boundary and metrology method.
- Metered source energy, storage charge/discharge, delivered energy, losses/auxiliaries and fuel.
- Explicit unserved energy and disturbance/recovery evidence.
- For storage-equipped v0.2 missions, a terminal storage-energy floor is mandatory and must be at least the measured starting stored energy within the declared state tolerance; explicit depletion accounting is a separate benchmark contract.
- Frozen acceptance limits and evidence lineage before the run.

This blocks a common comparison error: a hybrid system can appear to save fuel by consuming energy already stored in its battery. The test must restore that energy or debit the depleted energy under a separately agreed accounting method. This evaluator enforces a terminal-state floor at or above the measured initial stored energy for storage-equipped missions. The paired benchmark harness must bind baseline and candidate missions to the same service contract.

## 2. Energy-boundary contract

All bus-energy terms below must be measured at the same declared common service-bus boundary:

    E_source + E_storage,discharge = E_delivered + E_storage,charge + E_aux+loss

The per-interval absolute residual is:

    r_E = abs(E_source + E_storage,discharge - E_delivered - E_storage,charge - E_aux+loss)

The declared energy-balance tolerance must be frozen before the run and justified by the metrology/uncertainty evidence. Mycelix owns calibration, channel identity, timebase and correlated uncertainty; this module consumes the resulting evidence state and tolerance. It does not invent an uncertainty distribution or treat meter labels as authenticated truth.

Service accounting is separate:

    E_demand = E_delivered + E_unserved

Unserved energy is lost service, not energy supplied to the bus. Service-ledger residuals and bus-energy residuals are separate gates.

### Storage-state guard

The fields stored_energy_start_kwh and stored_energy_end_kwh are measured at the storage-internal state boundary; charge and discharge energy are measured at the common bus boundary. The evaluator enforces only conservative direction/bounds:

- Charging interval: stored energy cannot fall beyond tolerance and the internal stored-energy increase cannot exceed bus-side charge energy plus tolerance.
- Discharging interval: stored energy cannot rise beyond tolerance, and bus-side discharged energy cannot exceed the observed internal stored-energy decrease plus tolerance. Internal depletion may be larger than bus output because of storage/conversion losses; estimating that efficiency is outside this evaluator.
- Idle interval: stored energy cannot drift beyond its declared tolerance.
- A single aggregate interval may not contain both charge and discharge; split the interval at the direction change.
- Adjacent interval storage state must reconcile; the mission must have full, contiguous time coverage.

These checks are intentionally not an efficiency model. They do not estimate battery SOC, SOH, electrochemistry, capacity fade, thermal runaway risk or converter loss. The chosen intervals and internal/bus boundary must be explicit in a real test.

## 3. Evidence eligibility versus service acceptance

The output has two distinct decisions:

1. **evidence_eligible**: configuration digest, qualification currency, measurement status, calibration, timebase, channel manifest and capture completeness are current; service and bus ledgers close within their frozen tolerances; required disturbance observations are present.
2. **service_requirements_met**: only populated when evidence is eligible. It compares minimum demanded service, unserved energy/fraction, terminal storage energy, fuel intensity and configured disturbance/recovery/power-quality observations against the predeclared limits.

An ineligible measurement run receives no service verdict (null), not a false "pass" and not a score that could be mistaken for acceptable operation. A well-evidenced run can be service-failing; that is a meaningful result and remains visible.

The report retains the measurement-run ID, raw-capture digest, channel-manifest digest, qualified/observed configuration digests, calibration/timebase/uncertainty references and each event's measured quality/recovery evidence reference.

## 4. Implemented metrics

The current Rust evaluator computes:

- Mission duration and interval/event counts.
- Demanded, delivered and unserved energy; unserved fraction; a positive minimum demanded-energy gate prevents zero-load 'passes'.
- Source, storage discharge and charge, and auxiliary/loss energy.
- Initial, terminal and net-change stored energy.
- Fuel consumed and fuel per delivered kWh when delivered energy is non-zero.
- Worst interval and aggregate bus-energy residual; worst service-ledger residual.
- Maximum observed recovery time, voltage deviation and frequency deviation where those observations exist.
- Frozen-configuration and measurement-lineage gates.

It is an arithmetic/evidence audit over caller-supplied measured values. It does not parse raw waveforms, authenticate hashes, inspect calibration certificates, calculate protection selectivity, execute a controller or verify the physical truth of any sensor.

## 5. Benchmark protocol and targets

### Baseline pairs

For generator-only versus generator+BESS (and later other configurations), pair candidate and baseline on:
- Identical load demand, interval boundaries, mission duration and criticality.
- Identical service boundary, site/environmental stratum and load-start events.
- Equivalent source/load power-quality acceptance limits.
- Equivalent initial and terminal usable storage energy. This evaluator requires a storage-equipped candidate to finish with at least its initial measured stored energy (within tolerance). A depletion-debit study requires a separate, preregistered accounting contract/version; it must not silently bypass this gate.
- Identical fuel measurement boundary and treatment of auxiliaries.
- Frozen data-quality, uncertainty and missing-data rules.

Do not compare a small low-voltage converter against a 100–1,000 kVA containerized BESS as though they provide the same service. Compare component-to-component at the component tier, and only compare full systems after matching the service envelope.

### Research benchmark gates

These are programme targets, not current product results:

| Gate | Target / decision rule |
|---|---|
| Accounting integrity | Every interval closes both service and bus-energy ledgers within pre-registered uncertainty/tolerance; any unexplained energy creation or missing interval invalidates the run |
| Service parity | Candidate does not worsen the pre-registered unserved-energy, voltage/frequency, recovery or safety/quality gates versus the matched baseline |
| Terminal energy fairness | Candidate meets the same terminal usable-energy obligation as its starting state, within metrology uncertainty |
| Fuel efficiency | Compare litres per delivered useful kWh, stratified by load factor and duty cycle; target at least 15% median reduction in the specifically selected intermittent-load pilot, with no worse service or terminal energy. This is an internal research target, not a forecast or achieved result |
| Generality | Repeat across low/intermittent, variable motor-load and sustained-load strata; report paired per-episode outcomes and cluster-aware confidence bounds, not a single best day |
| Reliability/recovery | Fault/source-loss/restart/transfer scenarios must be independently specified and physically measured; simulation remains a separate evidence class |
| Lifecycle | Report usable energy, round-trip efficiency, auxiliary consumption, fuel, battery throughput, degradation, service/repair/requalification time, spares and total cost per delivered useful kWh |

A 15% target is deliberately a test hypothesis for the first intermittent-load pilot, not a claim to beat every manufacturer's best-case hybrid result. A manufacturer claim such as "up to" fuel savings is a product-specific upper-bound marketing statement unless the load profile, baseline, end SOC, environment and measurement method are disclosed and matched. The confirmatory target and required sample size must be preregistered before collecting comparison data.

### Metrics that must remain separate

Do not collapse the benchmark into one weighted score. Keep non-compensatory gates for:
- Electrical safety, protection and isolation.
- Power quality and interruption/recovery.
- Service delivery and energy accounting.
- Thermal/environmental limits.
- Data rights, metrology and provenance.

Only after those gates pass should operational cost, fuel, noise, lifecycle or repairability be ranked as a Pareto comparison.

## 6. Standards and benchmark references

Applicability must be determined from the final product boundary, installation, voltage class, jurisdiction and market route. Referencing a standard is not evidence of conformity.

- **DOE/PNNL, PNNL-26492, Energy Storage System Performance Test Plans and Data Requirements (2017):** useful test-design precedent for stored-energy capacity, AC/DC energy boundaries, auxiliaries, round-trip efficiency, response/ramp tests and application duty cycles. It is a test-plan reference, not a current product certificate or a universal portable-power requirement.
- **ISO 8528-5:2025 (Edition 6):** current published edition for design/performance criteria of reciprocating-engine AC generating sets; the 2022 edition is withdrawn. Use the performance class and customer acceptance limits appropriate to the actual generator/site. [ISO catalogue](https://www.iso.org/standard/85960.html)
- **ISO 8528-6:2023:** test methods for characterizing an entire reciprocating-engine AC generating set; its scope also describes simulation methods for assessing specified capability, but a model result must remain separately labelled from our measured system-service evidence. [ISO catalogue](https://www.iso.org/standard/71010.html)
- **IEC 62933-5-2:2025:** electrochemical EES system safety across lifecycle, specifically grid-integrated EES scope. Do not automatically treat it as the whole compliance route for a portable generator/BESS package.
- **UL 9540A, Edition 6 (published 2026-03-13):** a thermal-runaway fire-propagation test method. It is not a round-trip-efficiency or fuel-savings benchmark, and completing the method is not itself a product listing/certification.
- Product-specific South African SANS/NRCS/LOA and installation requirements remain to be determined for the eventual configuration and market route.

## 7. Cross-project ownership

- **Symtropy:** owns physical-model replay, time-dependent power-system behavior, topology and disturbance/fault scenario simulation. Simulated trajectories must never be relabelled as measurements.
- **Mycelix:** owns instrument/channel identity, calibration, timebase, uncertainty/correlation and evidence invalidation. This module receives the evidence contract; it is not a metrology authority.
- **Symthaea:** can rank the next experiment by expected decision value and identify missing evidence. It does not set safety limits, qualify a component or authorize dispatch.
- **Sol-Atlas:** owns site/resource/spatial and environmental context, including altitude, ambient, access, logistics and local source constraints.
- **Luminous Platform:** composes product configuration, mission profile, source/port capabilities and qualification/evidence reports.

## 8. Validation and claim ceiling

- The synthetic fixture is illustrative only; it contains no physical site or vendor measurements.
- Rust tests are authored but must not be described as executed until a Rust toolchain/runner reports the exact commit SHA and a successful run.
- A parsed JSON Schema is not a full schema-validator pass.
- No hardware prototype, HIL, fault injection, laboratory report, certification, field trial, fuel saving, service guarantee or SOTA superiority is claimed.
- Claim ceiling: MEASURED_MISSION_SERVICE_AUDIT_ONLY_NOT_DISPATCH_SAFETY_OR_PHYSICAL_QUALIFICATION.
