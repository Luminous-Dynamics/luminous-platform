# Power Fabric industry benchmark programme — 12 November 2026

**Parent:** [#419 — matched-service portable industrial power and modular protection](https://github.com/Luminous-Dynamics/luminous-platform/issues/419)  
**Architecture:** [#413](https://github.com/Luminous-Dynamics/luminous-platform/issues/413) · [draft PR #414](https://github.com/Luminous-Dynamics/luminous-platform/pull/414)

**Status:** vendor/literature baseline and research protocol only. No Luminous physical performance or superiority result is claimed.

## 1. Competitive question

Do not ask whether Power Fabric beats a competitor's headline kW or kWh. Ask:

> At the same useful-service boundary, load trace, site/environmental conditions, source availability and recovery obligation, which architecture provides more reliable, lower-cost and better-evidenced service?

The current 500 W PEBB-LV-01 laboratory candidate cannot compete with complete mobile ESS products on nameplate scale. The first differentiator should be component quality, protection, repairability, evidence and integration. Full-system comparison should integrate appropriately qualified commercial batteries/inverters/generators until a Luminous system reaches that scale.

## 2. Vendor baseline registry

| Product | Published configuration in this snapshot | Important distinction |
|---|---|---|
| Caterpillar XES120 (50 Hz) | 72 kW / 90 kVA at 25 °C; 127.9 kWh LFP / 48 Vdc; 400/230 V, 50 Hz; 1.5× rated overload for 30 s; 2× rated current short-circuit rating; inverter derate 96% at 30 °C / 90% at 40 °C / 82% at 50 °C; IP55 | Compact mobile hybrid ESS. Vendor settings allow genset-start thresholds around 57.6–64.8 kW fast (0–30 s) and 54.0–61.2 kW slow (60–600 s); these are control settings, not measured service guarantees |
| Atlas Copco ZBC 250-575 | 250 kVA nominal; 575 kWh nominal; 250 kW continuous mode; 400 V AC; 672–864 V DC battery bus; LFP, 0.5C; recommended 90% DoD / 70% EOL; 2 h discharge autonomy at rated power; IP54 | Large construction/industrial configuration. kVA rating is not the continuous kW rating; autonomy is vendor-published, not matched POC field measurement |
| Atlas Copco ZBC 300-300 | 300 kVA; 308 kWh | Higher-power, shorter-energy model |
| Atlas Copco ZBC 500-250 | 500 kVA; 246 kWh | kVA is not interchangeable with kW; peak and continuous ratings must remain distinct |
| Atlas Copco ZBC 1000-1200 | 1,000 kVA nominal; 1,200 kWh nominal; 800 kW continuous mode; 400 V AC; 672–864 V DC; LFP, 1C; 90% recommended DoD / 70% EOL; 1 h at rated power, 1.3 h at 75%, 2 h at 50%, 4 h at 25% in the vendor autonomy table; IP54 | Container-scale product, not a portable service-node comparator. Use the technical table units; do not repeat the web-page marketing phrase “1.2 MW energy capacity” as an energy unit |
| ABB SACE Infinitus | Fully IEC 60947-2-certified solid-state circuit-breaker; up to 2,500 A / 1,250 V DC; solid-state interruption plus electromechanical isolation; S/H/V variants distinguish di/dt regimes (<80 A/µs, <100 A/µs, and unlimited with external rise-time limiter) | Mature industrial DC breaker reference. Preserve variant/test conditions; not an eFuse or a 48 V auxiliary branch module |
| TI TIDA-050090 | 54 V / 5 kW eFuse reference design; one TPS1689 + five TPS1685; TI reports 100 A RMS at 55 °C without external airflow | Vendor reference board for testing/performance validation, not a commercial breaker or independent Luminous result |

Sources:
- Cat XES120 South Africa product: https://www.cat.com/en_ZA/products/new/power-systems/electric-power/battery-energy-storage-systems/119844.html
- Cat XES120 (50 Hz) technical sheet LEHE21362-03: https://emc.cat.com/n/api/pubdirect?media_string_id=LEHE21362-
- Atlas Copco ZBC South Africa: https://www.atlascopco.com/en-za/construction-equipment/products/energy-storage-systems/medium
- ABB Infinitus: https://www.abb.com/global/en/areas/electrification/low-voltage/circuit-breakers/solid-state-circuit-breakers/sace-infinitus
- TI TIDA-050090: https://www.ti.com/tool/TIDA-050090

The Atlas Copco South Africa vendor table also lists LFP batteries, a 672–864 V DC battery bus, recommended 90% depth of discharge, 70% end-of-life capacity, and variant-specific discharge C-rates (0.5C, 1C or 2C). These are vendor-published design values, not independent usable-energy measurements or a guaranteed field life. Preserve variant-specific values: do not combine the maximum power from one model with the maximum energy, C-rate or temperature rating of another. Keep kW/kVA, nominal/peak/continuous duration, battery-side energy and usable AC energy as separate quantities.

## 3. Grid-forming and conformity baseline

IEEE 2800-2022 is the active transmission/sub-transmission IBR interconnection-performance standard. IEEE 2800.2-2026, published 30 June 2026, provides recommended plant-level conformity-assessment practices for demonstrating conformance with IEEE 2800. Neither alone proves every GFM behavior. A 2026 review notes that phase-angle-jump response, GFM inertia behavior, autonomous island formation, GFM/GFL mode transition and black-start need supplementary GFM-specific or project-specific criteria.

Sources:
- IEEE 2800-2022: https://standards.ieee.org/ieee/7003/10453/
- IEEE 2800.2-2026: https://standards.ieee.org/ieee/2800.2/10616/
- Tan et al., *Review of Testing and Validation Methodologies for Grid-Forming Performance of Inverter-Based Resources*, Energies 19(20), 4966, published 19 October 2026: https://www.mdpi.com/1996-1073/19/20/4966

## 4. Benchmark ladder

### Tier A — component parity

Compare isolated bidirectional DC/DC, eFuse/SSPC/hybrid protection, local control/measurement, magnetics/thermal package and connectors. Freeze voltage/current, direction, load, ambient, cooling, measurement plane and revision. Report full maps, not peak points.

### Tier B — source-to-load subsystem

Measure the entire chain: source/cable loss, all conversion stages, distribution/protection loss, fans/pumps, controls/standby and storage round-trip energy. State the boundary; do not multiply unrelated peak-efficiency values from different devices and present them as a measured system result.

### Tier C — useful-service episode

Use the existing field-mission-profile schema and compare:
1. Generator-only reference.
2. Commercial BESS/inverter where viable.
3. Generator + commercial BESS hybrid.
4. Future Luminous-integrated candidate.

Bind identical loads, time series, priorities, ambient, source availability, mission duration, reserve and end-state obligations. Measure net energy delivered at load terminals, fuel per delivered useful kWh, unmet energy/work, power quality, motor starts, source transfers, generator start/stop/low-load dwell, battery SOC/temperature/throughput, recharge, rebound, noise and restoration.

A candidate cannot claim an efficiency win by shedding required work, missing a deadline, changing load, or ending with a depleted battery that the baseline did not deplete.

### Tier D — resilience/interoperability

Compose Symtropy's existing power-system digital twin and resilience owners. Test source/grid loss and return, generator trip/start failure, battery/inverter loss, shared bus/cooling/control failures, load steps/motor starts, island transition, black-start, resynchronization, communication loss, failed-short semiconductor, isolation and repair/requalification. Simulation, controller-HIL, power-HIL, laboratory and field evidence are separate tiers.

## 5. Required measurement contract

Every scored run must bind:
- hardware/PCB revision, serial, BOM and critical component lot;
- firmware/control digest, wiring, connector, busbar, enclosure and cooling revision;
- mission/load trace, source configuration, site and environment;
- exact procedure/revision, calibrated instrument/channel, bandwidth/range/uncertainty and timebase;
- raw data digest, analysis version, fuel/tariff/carbon-factor provenance and data rights.

Publish outcomes together: useful delivered energy, unserved energy, fuel, power quality, dynamic response, battery end state, protection/let-through/isolation, recovery/rebound, availability, repair time, lifecycle cost and evidence completeness. Safety, isolation, service fulfilment, power quality and evidence integrity are hard gates; they cannot be offset by a higher weighted average.

## 6. Research targets (not achieved results or standards limits)

- Zero false-available outputs across the frozen deterministic hard-negative and declared common-cause/cut-set corpus.
- Zero hidden workload/SLA deficits; all unserved service and failed/aborted runs remain visible.
- 100% exact configuration, instrument/calibration, procedure, timebase and raw-data lineage.
- If claiming a 0.5 percentage-point efficiency advantage, design repeatability and uncertainty to resolve that difference; otherwise label the result inconclusive.
- Hold out complete sites, assets, operating clusters and time windows; do not randomly split adjacent samples from one event.
- Score the full service episode through isolate, repair, requalification, restore and SOC/thermal recovery, including rebound.
- Publish p50/p95/p99 dynamic metrics only where sample volume supports them, plus maxima and failure cases.
- No blanket percentage improvement over Cat or Atlas before a matched duty cycle and independent measurements exist.

## 7. Compose Luminous owners, do not duplicate them

| Owner | Contribution |
|---|---|
| Symthaea | Decision-impact experiment selection (#7803); choose the next test that could change a topology/product decision under a fixed lab budget |
| Symtropy | Power-system digital twin (#3400), resilience (#3418), topology (#3409), matched-service FFER (#3417); replay generator/storage/load and disturbances |
| Sol-Atlas | Energy portfolio/site (#293/#662) and spatial/infrastructure dependency (#692); location, source access and site context |
| Mycelix | Metrology (#6034), electrical channel taxonomy (#6029), correlated uncertainty (#6048), invalidation (#6042), timebase (#6054), condition applicability (#6065) |
| Luminous Platform | Product configurations, mission profiles, capability envelopes, contingency composition, benchmark manifests and evidence-qualified results |

## 8. Immediate execution

1. Keep the existing field-mission-profile schema as the site/load mission input contract.
2. Use the mission-energy envelope (#422) to compose owner-qualified interval-average source capability, storage limits, conservative uncertainty and end-of-mission reserve.
3. Bind each replay to Symtropy model/trace revisions and each measured observation to Mycelix instrument/timebase/calibration evidence.
4. Use Symthaea's decision-impact selector to prioritize specialist laboratory tests.
5. Bind package, connectors, harness, cooling and service-access revisions through Sol-Atlas's product engineering thread.
6. Preregister boundaries, primary metrics, exclusions, uncertainty method and hard gates before any physical A/B run.

## 9. Claim boundary

The competitor values above are vendor-published specifications, not independent measurements. The Power Fabric branch currently contains research documents, schemas, synthetic fixtures and software screening tools; it does not contain a fabricated PCB, qualified power stage, field product, accredited test result, protection certificate or independently reproduced performance advantage. CI/test runners have not verified the current exact head. No energized battery, mains/grid, high-energy fault or field test is authorized by this benchmark document.


## 10. Mission energy, storage and reserve comparison

[Issue #422](https://github.com/Luminous-Dynamics/luminous-platform/issues/422) and
[MISSION_ENERGY_ENVELOPE.md](./MISSION_ENERGY_ENVELOPE.md) introduce an interval-energy contract
for mission-level service comparison. This does not duplicate Symtropy's generator, inverter or
hybrid-plant physics. It consumes the time-weighted interval-average output from that owner, applies
conservative source measurement/model uncertainty, and accounts for one aggregate store's capacity,
SOC, charge/discharge power and efficiencies.

The key competitive control is the **equal terminal-state obligation**: compare generator-only,
generator+BESS and BESS-only against the same useful load, environment, outage schedule, initial SOC
and final reserve. A hybrid system cannot claim fuel/efficiency superiority by discharging a battery
and leaving the next operator with a depleted system. Report the SOC trace, source energy, battery
energy withdrawn/added, curtailed energy, delivered service and every unserved segment.

The synthetic two-segment fixture demonstrates arithmetic only (1,000 Wh source surplus charged at
90%, followed by 800 Wh of storage-delivered service at 90% discharge efficiency). It is not a
measured system or performance result.

Standards/test-method references newly added for this work:
- [ISO 8528-5:2025](https://www.iso.org/standard/85960.html) for the RIC-engine/AC-generator-set subsystem.
- [IEC TS 62933-2-2:2022](https://webstore.iec.ch/en/publication/64570) for EES application duty cycles/performance testing.
- [IEC TS 62933-2-3:2025](https://webstore.iec.ch/en/publication/77334) for site-operation performance assessment after commissioning.
- [IEEE 2030.3-2016](https://standards.ieee.org/ieee/2030.3/5076/) and [IEEE 1679-2020](https://standards.ieee.org/ieee/1679/7716/) as scoped storage test/characterization references.

These references do not by themselves qualify a portable hybrid product. Product classification, South
African SANS/NRCS applicability, battery safety, converter safety, protection, installation and
accredited physical testing remain separate release gates.


## 11. Lifecycle-adjusted service cost and repairability

[Issue #424](https://github.com/Luminous-Dynamics/luminous-platform/issues/424) opens the next benchmark layer: repeated matched missions across battery capacity/power fade, calendar/cycle ageing, fuel and energy costs, replacement, maintenance, repair turnaround and requalification. The comparison denominator remains delivered critical service and completed missions—not nominal battery kWh or one-cycle efficiency.

This work consumes qualified state-of-health/capacity/power trajectories from the appropriate battery model and Mycelix evidence; it does not create a new electrochemical ageing model. Unknown degradation is not zero degradation. Beginning-, mid- and end-of-life service are reported with replacements, downtime, repair labour, spares and uncertainty retained. No lifecycle advantage is claimed until a repeated, matched and independently measured mission dataset exists.
## 12. Power-duration event / motor-start benchmark

[Issue #425](https://github.com/Luminous-Dynamics/luminous-platform/issues/425) and
[POWER_DURATION_EVENT_ENVELOPE.md](./POWER_DURATION_EVENT_ENVELOPE.md) add a peak-power/time-window
comparison beside the interval-energy mission comparison. The benchmark must include motor starts,
load steps/rejections, source loss/transfer, storage bursts, repeated events and the common bus/output
path. Record power support by duration, event SOC/SOH/temperature, peak/RMS voltage/current, voltage
and frequency nadir, recovery/settling, process outcome, and independent protection behaviour.

The new evaluator does not use one large surge multiplier: the source's guaranteed event-window power
is combined with state-bound storage support, then capped by a separately qualified common-path
limit. A critical event has its own local unserved-energy limit. It is a software screen, not a
waveform/motor model or proof of voltage/frequency quality.

For test-method selection, screen IEC 61000-4-11:2020 (up to 16 A mains current/phase) or IEC
61000-4-34:2005+A1:2009+A2:2025 (above 16 A/phase) for equipment immunity to specified voltage dips
and interruptions; IEC 61000-4-4:2012 for EFT/burst immunity. These do not certify motor-start or
generator-source performance. Use ISO 8528-5:2025 for the applicable RIC-engine/AC-generator-set
subsystem. Apply only where the product boundary fits.



## 12. Benchmark evidence passport and site-operation follow-up

[Issue #429](https://github.com/Luminous-Dynamics/luminous-platform/issues/429) and
[BENCHMARK_EVIDENCE_PASSPORT.md](./BENCHMARK_EVIDENCE_PASSPORT.md) add an evidence-bearing run
manifest and pairwise matched-comparison gate. The comparison keys bind exact primary/auxiliary
POCs, useful-service boundary, demand/event/environment/source/fuel context, initial storage/reserve,
test/data-processing/timebase methods, mission/reporting windows, calibration method and metric
measurand scope. Product configuration digests are preserved separately because comparators are
expected to differ.

IEC TS 62933-2-3:2025 is an important site-operation reference. Its public preview distinguishes
operational-data assessment from precise laboratory testing; it recommends representative sample
events and records, and includes power/voltage/current/energy, fault/maintenance and dispatch
history in the operational data picture. It also lists actual storage capacity, input/output power,
round-trip efficiency, auxiliary consumption, available hours, equivalent operation factor and
availability/outage indicators among relevant performance items. Accordingly, Power Fabric's run
passport keeps sample selection, preprocessing, operational history, auxiliary POC and timebase
provenance explicit. Site-assessment evidence is a field-performance view, not a substitute for
controlled tests when accurate numeric characterization is needed.

The code returns three distinct gates—structural match, physical measurement eligibility and
independent claim eligibility. A synthetic pair can test the first gate but not the latter two.
No physical superiority claim is made.


## 13. Dispatch-controller benchmark

[Issue #432](https://github.com/Luminous-Dynamics/luminous-platform/issues/432) and
[DISPATCH_CONTROLLER_BENCHMARK.md](./DISPATCH_CONTROLLER_BENCHMARK.md) open the controller-policy
qualification layer. It compares fixed schedules, transparent rules, constrained optimisation and
MPC/robust-MPC policies over complete matched mission trajectories, including forecast error, source
loss/start failure, storage derating, auxiliary/cooling common causes, stale telemetry and recovery.

The controller benchmark consumes source, storage, common-path and mission constraints from their
owners; it does not create a competing physical plant model. Service, feasibility/safety, resilience,
resource/lifecycle, controller quality and evidence/replay remain separate scoreboards. Critical
service, qualified-envelope, telemetry and terminal-reserve failures are non-compensatory.

The versioned schema and synthetic positive/negative corpus are under
`schemas/dispatch-controller-*`; the typed in-memory semantic validator is registered in
`measurement-audit/src/dispatch_controller_corpus.rs`. These are offline fixtures, not controller
results or physical measurements. Alongside IEEE 2030.7-2017 and IEEE 2030.8-2018, the programme
includes IEC TS 62898-3-5:2026 for HIL, commissioning and periodic testing of MMCS/MEMS.
The IEEE P2030.7/P2030.8 revisions remain active projects in this snapshot. No standards-conformity
or dispatch-superiority claim is made.


### IEC TS 62898-3-5:2026 — newly published controller test reference

The benchmark now includes [IEC TS 62898-3-5:2026](https://webstore.iec.ch/en/publication/84175), published 11 June 2026. The IEC publisher states that it provides requirements for HIL, commissioning and periodic testing to verify and quantify microgrid monitoring, control and energy-management systems in grid-connected or isolated microgrids.

This is a more direct test-method anchor for the controller qualification work than treating controller tests as an extension of the storage-energy tests. Map its licensed requirements into separate HIL, commissioning and periodic-test records. Keep IEEE 2030.7-2017 (controller functional specification) and IEEE 2030.8-2018 (controller test procedures/metrics) in the crosswalk; their P2030.7/P2030.8 revisions are active projects, not yet published replacement standards in this snapshot.

Sources:
- IEC TS 62898-3-5:2026: https://webstore.iec.ch/en/publication/84175
- IEEE 2030.7-2017: https://standards.ieee.org/ieee/2030.7/5941/
- IEEE 2030.8-2018: https://standards.ieee.org/ieee/2030.8/6169/
- P2030.7: https://standards.ieee.org/ieee/2030.7/12022/
- P2030.8: https://standards.ieee.org/ieee/2030.8/12543/


### Controller test families and measurement uncertainty update

The controller crosswalk now enumerates the MEMS HIL families (dispatch optimisation, forecasts, demand-side integration, flexible-resource management) separately from MMCS HIL functions (anti-maloperation, voltage/frequency regulation, switching, mode transition, P/Q control, islanding, black start and end-to-end HIL before commissioning) listed in the public contents of IEC TS 62898-3-5:2026. See [DISPATCH_CONTROLLER_BENCHMARK.md](./DISPATCH_CONTROLLER_BENCHMARK.md). Detailed test conditions and acceptance values require licensed-text review.

The metrology crosswalk also records JCGM 100:2008/Amd.1:2026 and JCGM GUM-5:2026 from the BIPM/JCGM catalogue. Passport PR #430's conservative uncertainty-bound sum remains diagnostic only; nonlinear/correlated uncertainty calculation and method receipts remain Mycelix-owned (#6048). Platform consumes that result and does not create a second uncertainty engine.


## 14. Vendor baseline refresh — 12 November 2026

A current manufacturer-source check adds operating-envelope details that should shape the matched benchmark rather than being collapsed into nameplate numbers.

### Cat XES120 50 Hz

The Cat technical sheet lists a 72 kW / 90 kVA inverter rating at 25 °C, 127.9 kWh LFP at 48 Vdc, 1.5× rated overload for 30 seconds, 2× rated current short-circuit rating, and inverter derating of 96% at 30 °C, 90% at 40 °C and 82% at 50 °C. It also lists configurable generator-start thresholds (fast and slow) and their delays. These settings are useful controller-test inputs, not independently measured guarantees of delivered customer service.

Source: Cat XES120 50 Hz technical sheet LEHE21362-03: https://emc.cat.com/n/api/pubdirect?media_string_id=LEHE21362-

### Atlas Copco ZBC container range

The South African manufacturer table distinguishes nominal kVA, nominal kWh and continuous kW. For the ZBC 1000-1200 it lists 1,000 kVA nominal, 1,200 kWh nominal and 800 kW continuous mode; the autonomy table lists 1 h at rated power, 1.3 h at 75%, 2 h at 50% and 4 h at 25%. The same table lists 90% recommended DoD and 70% EOL. For the ZBC 250-575 it lists 250 kVA nominal, 575 kWh nominal and 250 kW continuous mode. The webpage's marketing phrase “1.2 MW energy capacity” is not an energy unit; use the technical table's 1,200 kWh.

Source: Atlas Copco South Africa ZBC technical table: https://www.atlascopco.com/en-za/construction-equipment/products/energy-storage-systems/medium

### Benchmark implications

1. Match the inverter/continuous kW mode and event-duration envelope separately from nominal kVA.
2. Test at the manufacturer-declared ambient/derating breakpoints and measure delivered POC service; do not assume the 25 °C rating at 50 °C.
3. Compare controller generator-start threshold, delay and service continuity under identical load ramps and forecast information.
4. Request auxiliary/HVAC energy and net AC output data from vendors where the public table does not disclose them; do not infer auxiliary consumption from container ratings.
5. Reconcile actual discharge energy, start/stop events, final SOC/reserve, thermal state and recovery rather than dividing nominal kWh by rated kW and calling it field autonomy.

All figures above remain vendor-published technical claims. They are test-design references, not independent matched measurements or evidence that Power Fabric is better or worse.
