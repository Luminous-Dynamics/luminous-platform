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
| Caterpillar XES120 (50 Hz) | 72 kW / 90 kVA; 127.9 kWh; IP55; 400/230 V, 50 Hz inverter; 0–50 °C operating range; variable-speed forced-air cooling | Compact, mobile hybrid ESS reference; published ratings are vendor specifications, not matched independent measurements |
| Atlas Copco ZBC 250-575 | 250 kVA; 575 kWh; 400 V AC; 672–864 V DC battery bus; −20 to 50 °C; IP54 | Large construction/industrial configuration |
| Atlas Copco ZBC 300-300 | 300 kVA; 308 kWh | Higher-power, shorter-energy model |
| Atlas Copco ZBC 500-250 | 500 kVA; 246 kWh | kVA is not interchangeable with kW; peak and continuous ratings must remain distinct |
| Atlas Copco ZBC 1000-1200 | 1,000 kVA; 1,200 kWh; vendor table lists 800 kW continuous mode | Container-scale unit, not a portable service-node comparator |
| ABB SACE Infinitus | ABB says fully IEC 60947-2-certified solid-state circuit-breaker; semiconductor interruption plus electromechanical isolation; up to 2,500 A / 1,250 V DC; interruption down to 15 μs | Mature solid-state breaker benchmark; not an eFuse or a 48 V auxiliary branch module |
| TI TIDA-050090 | 54 V / 5 kW eFuse reference design; one TPS1689 + five TPS1685; TI reports 100 A RMS at 55 °C without external airflow | Vendor reference board for testing/performance validation, not a commercial breaker or independent Luminous result |

Sources:
- Cat XES120: https://www.cat.com/en_US/products/new/power-systems/electric-power/battery-energy-storage-systems/119844.html
- Atlas Copco ZBC South Africa: https://www.atlascopco.com/en-za/construction-equipment/products/energy-storage-systems/medium
- ABB Infinitus: https://www.abb.com/global/en/areas/electrification/low-voltage/circuit-breakers/solid-state-circuit-breakers/sace-infinitus
- TI TIDA-050090: https://www.ti.com/tool/TIDA-050090

Atlas ZBC values vary by exact model. Do not combine the maximum power from one model with the maximum energy or temperature rating of another. Preserve kW/kVA, nominal/peak/continuous duration, battery-side energy and usable AC energy as separate quantities.

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

1. Keep the existing field-mission-profile schema as the mission input contract.
2. Add generator-only and generator+BESS synthetic matched episodes with independent arithmetic oracles.
3. Bind each replay to Symtropy model/trace revisions and each measured observation to Mycelix instrument/timebase/calibration evidence.
4. Use Symthaea's decision-impact selector to prioritize specialist laboratory tests.
5. Bind package, connectors, harness, cooling and service-access revisions through Sol-Atlas's product engineering thread.
6. Preregister boundaries, primary metrics, exclusions, uncertainty method and hard gates before any physical A/B run.

## 9. Claim boundary

The competitor values above are vendor-published specifications, not independent measurements. The Power Fabric branch currently contains research documents, schemas, synthetic fixtures and software screening tools; it does not contain a fabricated PCB, qualified power stage, field product, accredited test result, protection certificate or independently reproduced performance advantage. CI/test runners have not verified the current exact head. No energized battery, mains/grid, high-energy fault or field test is authorized by this benchmark document.
