# Power Fabric vendor-neutral controller industry crosswalk — v0.1

**Parent:** [Issue #442](https://github.com/Luminous-Dynamics/luminous-platform/issues/442)  
**Snapshot:** 12 November 2026  
**Status:** public vendor documentation and research-method crosswalk. Vendor descriptions are not matched test results.

## Context

Power Fabric now has:
- [#432 / PR #433] matched mission dispatch-controller scenarios and policy arms;
- [#434] metric-specific sampling adequacy and cluster-aware inference;
- [#440 / PR #441] replay-bound, non-compensatory episode outcomes and paired campaign comparison;
- existing #416 capability envelopes, #417 common-cause-aware multi-port analysis, #423 mission energy and #426 power-duration events.

The next need is a vendor-neutral industry crosswalk so that our benchmark compares against real controller architectures and known dispatch strategies without confusing marketing claims, time scales or validation tiers.

## Current product / software reference classes

Public-source snapshot (12 November 2026; these are product descriptions, not matched test results):

- **SEL powerMAX**: vendor describes sub-cycle islanding/resynchronization, protection and load shedding, centralized control and distributed relay-based control options, and integration with DERMS/ADMS: https://selinc.com/services/microgrids-power-management/microgrid-control-systems/
- **Schneider EcoStruxure Microgrid Operation / Microgrid Flex**: describes real-time topology, load sharing, DER/grid management, load shedding, black-start sequencing and relay/grounding management. Microgrid Flex distinguishes cloud dispatch-plan updates at 15-minute cadence from the local EMO-M controller's seconds-scale islanding/reconnection and off-grid control: https://productinfo.se.com/esxp_digital_apps_nema/mg/English/BM_ESXP_DG_Microgrid_NEMA_0001157650.xml/%24/ESXP_DG_MG_SD_Processing_0001125595 and https://productinfo.se.com/microgrid_guide/0100db2502_ecostruxure_microgrid-flex_design-guide_ansi-optimize/English/0100db2502-Microgrid%20Flex%20Design%20Guide-ANSI-Optimize%20%28bookmap%29_DD00741716.xml/%24/MGFlex_ANSI_Opt_Designguide_PowerManagementSystemattheEdge_EMO-M_DD01456778
- **Siemens SICAM Microgrid Controller**: vendor describes automatic islanding, fast active/reactive power control, demand response, load shedding and advanced dispatch; its published 2 GW capacity is a vendor capability claim, not a matched functional benchmark: https://www.siemens.com/en-us/products/microgrids/sicam-microgrid-control/
- **Eaton Power Xpert Microgrid Controller**: vendor documents automated sequencing, priority load shedding, islanding, frequency/voltage regulation, time-of-use and other energy-management strategies, and millisecond-timestamped event logging: https://www.eaton.com/content/dam/eaton/services/eess/eess-documents/eaton-eess-microgrid-controller-eaag-product-aid-pa027059en.pdf
- **OpenEMS**: open-source modular edge/backend energy-management stack with device-independent controllers, one-second execution cycles and a documented recommendation to keep controller logic below 100 ms: https://openems.github.io/openems.io/openems/latest/coreconcepts.html and https://openems.github.io/openems.io/openems/latest/edge/nature.d/io.openems.edge.controller.api.html
- **HOMER Pro/Grid**: established dispatch baselines include Load Following, Cycle Charging, Combined Dispatch, Generator Order Dispatch and MATLAB Link; the default Load Following and Cycle Charging strategies are described as not looking ahead: https://homerenergy.com/docs/knowledgebase/article/dispatch-strategy-and-controllers/

## Research evidence and gaps

The 2026 review of EMS decision-making identifies architecture-aware comparisons, islanded/offshore cases, fine-grained real-time re-optimization, battery-degradation economics and hardware validation as open gaps. It also concludes that classical optimization, MPC and learning-based policies do not dominate all objectives: https://www.mdpi.com/1996-1073/19/15/3560

IEC TS 62898-3-5:2026 explicitly includes HIL tests for dispatch optimization, forecasting, demand-side integration and flexible-resource management, alongside microgrid monitoring/control tests for anti-maloperation, voltage/frequency regulation, switching, mode transition, active/reactive power, islanding detection, black start and end-to-end behavior. Commissioning and periodic tests are separate stages. Confirm the exact normative detail against the licensed standard: https://webstore.iec.ch/en/publication/84175

## Paradigm shift: a three-timescale, architecture-aware qualification frontier

Do not compare an EMS day-ahead schedule to a sub-cycle relay response as if they were the same controller problem. Establish three linked but separately scored layers:

1. **Fast electrical / protection layer** — sub-cycle to milliseconds: island detection, current/voltage response, protection and fault interruption. Requires instrumented power/HIL or laboratory waveform evidence; supervisory EMS cannot claim these results.
2. **Edge supervisory control layer** — roughly 100 ms to seconds: device sequencing, load shedding, grid/island transition, setpoint tracking and local fallback. Use controller-HIL and measured I/O where available.
3. **Energy-management / dispatch layer** — seconds to 15-minute operating cycles and day-ahead schedules: forecasts, economic dispatch, reserve, flexible load, storage throughput and recovery. Use matched full-mission trajectories and lifecycle accounting.

The architecture dimension is another explicit factor: compare centralized, hierarchical/cloud-edge and distributed control under the same plant, workloads, information budget and policy logic. Include latency, dropped/reordered messages, partition, timebase drift and loss of central service. This responds directly to the architecture-aware benchmarking gap.

## Benchmark crosswalk and baseline arms

- Rule dispatch: HOMER Load Following / Cycle Charging / Combined Dispatch / Generator Order as functional strategy references; reproduce the rule semantics, not vendor implementation assumptions.
- Open modular edge EMS: OpenEMS control-cycle and component-controller architecture as an open software baseline.
- Commercial black boxes: SEL powerMAX, Schneider EMO-M/EMA, Siemens SICAM and Eaton Power Xpert, if a controlled lab or owner permits matched access. Treat marketing feature claims as test hypotheses, not results.
- Optimization baselines: LP/MILP deterministic oracle, rolling-horizon MPC, stochastic/robust MPC, and an explicitly labelled clairvoyant bound.
- Learning-based policy: RL or Symthaea advisory policy only with same observed information, same action constraints and deterministic runtime assurance shield; no model sees future information hidden from operational baselines.

## Proposed targets (research goals, not achieved results or standard limits)

- Zero false-available or false-PASS outcomes across the frozen hard-negative, cut-set, telemetry and comparison-integrity corpus.
- Every control timescale meets its declared deadline; as an initial one-second edge-cycle reference, target p99 controller-logic execution below 100 ms, with no missed hard control deadline. This is an OpenEMS-derived comparator target, not a universal vendor requirement.
- Zero hard service, qualified-envelope, protection or terminal-reserve violations in feasible reference scenarios.
- For reliability claims, publish successes/failures and a lower confidence bound using the independent unit that supports the claim; the issue #434 method applies. A 59/59 independent all-success sample only supports roughly a 95% one-sided 95% lower bound under iid assumptions; clustered/site data need cluster-aware methods.
- Demonstrate at least one preregistered Pareto improvement over the strongest feasible baseline in useful service per lifecycle input, with no hard-gate regression and a margin exceeding combined uncertainty.
- Hold out complete sites, seasons, assets, firmware/configuration clusters and disturbance families.
- Publish regimes where each baseline wins; do not claim a universal winner.

## Acceptance

1. Vendor/product descriptions and actual tested results are stored in separate evidence classes.
2. Each competitor row names control layer, timescale, architecture, functions, test access and claim ceiling.
3. The 3-layer benchmark maps to IEC TS 62898-3-5:2026 / IEEE 2030.8-2018 without copying normative standard text.
4. Product ratings and feature claims are not compared as measured performance.
5. Test scenarios cover both normal and failure operation, including black start, islanding/reconnection, protection/telemetry degradation and post-disturbance reserve recovery.
6. Centralized, hierarchical and distributed architectures are tested under matched information and plant conditions.
7. Per-layer result vectors remain separate; no composite score compensates across hard constraints.
8. Statistical/sample adequacy is bound to #434 and independent evidence/replay to #431.
9. No market leadership or SOTA superiority claim until independently measured matched tests exist.

## Ownership

- Platform: crosswalk, benchmark arms, scenario/outcome composition and claims.
- Symtropy: plant model and replay.
- Mycelix: metrology, evidence integrity and invalidation.
- Sol-Atlas: site/topology/environment context.
- Symthaea: adversarial case generation and experiment selection, not evidence authority.
- Specialist protection/control engineers and accredited laboratories: layer-specific test design and review.

## Claim ceiling

This issue creates a vendor-neutral comparison framework and public-source baseline register. It does not establish vendor performance rankings, Power Fabric product readiness, standards conformity or physical SOTA superiority.

## Claim ceiling

PUBLIC INDUSTRY CROSSWALK AND BENCHMARK DESIGN ONLY_NOT_VENDOR_RANKING_OR_PHYSICAL_SOTA
