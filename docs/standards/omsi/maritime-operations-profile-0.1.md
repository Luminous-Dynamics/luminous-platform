# OMSI Maritime Operations and Lifecycle Assurance Profile 0.1

**Status:** draft industry profile; not an adopted standard, product certification or regulatory approval  
**Date:** 2026-11-18  
**Owner issue:** [OMSI-MARITIME-001 #491](https://github.com/Luminous-Dynamics/luminous-platform/issues/491)  
**Portfolio parent:** [OMSI-INDUSTRY-001 #154](https://github.com/Luminous-Dynamics/luminous-platform/issues/154)  
**Cross-stack parent:** [Mycelix MARITIME-CROSS-STACK-INTEGRATION-001 #5968](https://github.com/Luminous-Dynamics/mycelix/issues/5968)

> This profile defines an evidence/interchange and operator-workflow boundary for maritime lifecycle systems. It does not certify vessels, approve designs, establish navigational safety, implement ship control, or replace flag, coastal, port, class, shipyard or independent engineering authorities.

## 1. Purpose and scope

Maritime engineering data is fragmented across requirements, CAD/CAE, product configuration, ERP/MES, yard schedules, supplier records, metrology/NDT, commissioning, vessel operations, maintenance, ports and environmental observations. The central problem is not the absence of another dashboard. It is the loss of identity, applicability, uncertainty, authority and causal lineage as information crosses these boundaries.

OMSI Maritime defines the platform contract for carrying those distinctions across the lifecycle:

`mission / requirement -> design revision -> configuration -> procurement -> fabrication -> inspection -> assembly -> commissioning -> operation -> maintenance / retrofit -> requalification -> reuse`

The profile is designed for staged adoption by civilian vessel builders, shipyards, vessel operators, ports, service providers, equipment suppliers and research programmes. A deployment must declare its vessel class, mission, flag, voyage/service, operating region, yard, supply-chain boundary, data owners and intended decisions. No universal maritime profile or threshold is assumed.

### In scope

- Loss-aware interchange of source-qualified lifecycle events and owner-native evidence references.
- Exact identity and revision binding for vessels, designs, serialized components, process plans, inspections, commissioning and operational profiles.
- Human review, approval-request, incident, effect-receipt and ambiguous-outcome reconciliation workflows using existing OMSI operation semantics.
- Cross-cutting cyber, PNT, communications, energy, supplier and environmental dependency visibility.
- Evidence-quality, interoperability, lifecycle and operational benchmark definitions.

### Out of scope

- CAD authoring, B-rep generation, CFD, hydrodynamics, FEA, structural strength, hydrostatics, stability or seaworthiness.
- COLREG interpretation, route planning, navigation, collision avoidance, ECDIS, ship control or remote/autonomous vessel control.
- Physical measurement truth, supplier truth, legal title, permits, class/flag-state approval or certification.
- A second GIS, evidence graph, asset registry, product passport, maintenance planner, simulation engine, shipyard scheduler or authorization broker.

Use owner-native components. If an owner cannot represent a required maritime semantic, raise a narrow adapter/contract gap before adding another generic abstraction.

## 2. The paradigm: the vessel is a maintained configuration, not a drawing

A named vessel is not enough to identify the physical system whose performance or safety is being discussed. The relevant identity is the exact as-built and maintained configuration, with its revisions, serialized components, material/process histories, installed software and bounded capability evidence.

The lifecycle thread should preserve the distinction between:

- product family, design revision, engineering configuration and individual serialized vessel;
- planned BOM, approved substitution, received lot, installed component and inspected component;
- nominal geometry, analysis geometry, exchanged CAD, as-built survey and repaired geometry;
- a work order marked complete, an observed operation, an independent measurement and an accepted postcondition;
- a supplier-signed assertion, authenticated issuer, independently checked record and verified physical condition;
- design qualification, class/flag approval, statutory authorization and operational permission;
- simulation, forecast, inferred state, provider claim and measured observation.

The profile must not collapse these into one `status`, `confidence`, `trust` or composite score.

### 2.1 Evidence-bearing references

Every material projection should preserve, where supplied and relevant:

| Dimension | Required semantics |
|---|---|
| Subject | Stable owner-native ID, subject kind, configuration/revision and relationship to parent/child artifacts |
| Issuer | Issuer identity, signing method, issuer authority/scope and verification result as separate facts |
| Artifact | Media/schema type, exact version, byte digest, source URI/reference, access/rights state and derivation lineage |
| Time | Event time, observation/effective time, source time basis/clock domain, ingestion/retrieval time and as-of query time |
| Measurement | Value, unit, coordinate/reference frame, method, instrument, calibration and uncertainty where applicable |
| Applicability | Vessel/yard/mission/operating profile, environmental envelope, model capability and excluded conditions |
| Evidence class | Measured, independently measured, provider-reported, documentary, inferred, forecast, simulated, synthetic or unknown |
| Disposition | Supported, contradicted, stale, withdrawn, missing, conflicting, not evaluated, unsupported or not comparable |
| Rights | License, terms, consent, permitted use, attribution and retention constraints |

A digest establishes byte identity relative to the bytes hashed; it does not authenticate the publisher or prove that the bytes describe a true physical event. An authenticated statement does not prove that a measurement was accurate. A rendered map, status badge or summary must remain reversibly linked to the underlying owner-native artifact and its source lineage.

### 2.2 Time and revision semantics

Do not order maritime truth solely by ingestion time. Late-arriving survey, inspection, warning, supplier and operational events must remain reconcilable by event time, effective interval, source time basis and correction lineage. A newer URL response is not automatically a newer valid revision. A missing update is unknown, not an affirmative safe/clear state.

Where clock domains cannot be correlated with admitted evidence, temporal ordering or latency claims must be `INDETERMINATE`. Preserve event time and observed-at time separately. Corrections supersede through explicit lineage; do not mutate history in place.

### 2.3 Configuration change and requalification

A substitution, software update, repair, rework, model update, sensor replacement or geometry change must identify its dependency impact. The profile should support impact queries across:
- affected requirements and design artifacts;
- purchased, received, fabricated, installed and serialized components;
- completed and in-progress work packages;
- inspection/NDT and commissioning results;
- operational modes, declared envelopes and service dependencies;
- maintenance instructions, spare compatibility and requalification obligations.

Only dependent qualifications should be invalidated, but no dependent qualification may silently survive a change outside its demonstrated applicability. The canonical configuration ledger and qualification semantics remain with their existing owners.

## 3. Cross-stack ownership and hand-off

| System | Owner-native responsibility | Maritime hand-off to OMSI |
|---|---|---|
| [Luminous Platform](https://github.com/Luminous-Dynamics/luminous-platform) | Profile admission, loss-aware interchange, operator workflow, authorized operation/effect boundary, reconciliation and cross-cutting assurance | Profile projections, review/incident/operation records, explicit authority state and verified postconditions |
| [Symthaea](https://github.com/Luminous-Dynamics/symthaea) | Mission/requirement reasoning, candidate vessel/hull geometry, engineering interpretation, model applicability, uncertainty and bounded decision support | Exact requirement/design/candidate identity, admission report, engineering decision brief and non-claims |
| [Symtropy](https://github.com/Luminous-Dynamics/symtropy) | Deterministic physical/resource/schedule execution where supported, scenario replay, independent feasibility evaluators and run receipts | Exact scenario/configuration/run identity, supported/unsupported observable census, replay digest and evaluator disposition |
| [Mycelix](https://github.com/Luminous-Dynamics/mycelix) | Participant identity/recognition/authority, provider commitments, configuration and product lifecycle identity, custody/provenance and inspection/qualification records where assigned | Owner-native references, commitments, signed claims, custody/inspection records and correction lineage |
| [Sol-Atlas](https://github.com/Luminous-Dynamics/sol-atlas-leptos) | Spatial/temporal source projection, geospatial assets, coverage, CRS/datum, uncertainty, rights and provenance | Source-pinned geometry/coverage/time/context and reversible spatial projection |

### 3.1 Existing work to compose

- [Mycelix #5968 — cross-stack maritime integration](https://github.com/Luminous-Dynamics/mycelix/issues/5968) is the integration parent; do not create a competing monolith.
- [Symtropy #2048 — shipbuilding co-design](https://github.com/Luminous-Dynamics/symtropy/issues/2048), [#2050 — shipyard space-time verification](https://github.com/Luminous-Dynamics/symtropy/issues/2050), [PR #2052](https://github.com/Luminous-Dynamics/symtropy/pull/2052) and [PR #4204 — public ship-block corpus admission](https://github.com/Luminous-Dynamics/symtropy/pull/4204) own the shipyard benchmark seam. PR #2052 is a limited synthetic rectangle/time verifier scaffold, not a yard scheduler or physical-lifting model. PR #4204 documents source admission; its own discussion states raw workbook bytes were not retrieved and numerical reproduction is not claimed.
- [Symthaea #12302 — generative hull geometry](https://github.com/Luminous-Dynamics/symthaea/issues/12302), [#11513 — validated multi-fidelity design](https://github.com/Luminous-Dynamics/symthaea/issues/11513), [#7444 — marine freight/co-design](https://github.com/Luminous-Dynamics/symthaea/issues/7444), [#11049 — marine serviceability](https://github.com/Luminous-Dynamics/symthaea/issues/11049) and [#10966 — task-specific habitability/seakeeping](https://github.com/Luminous-Dynamics/symthaea/issues/10966) own vessel and human-experience engineering work.
- [Mycelix #6154 — marine product lifecycle](https://github.com/Luminous-Dynamics/mycelix/issues/6154) owns configuration, manufacturing, inspection and repair lineage. This profile does not create another product-passport ledger.
- [Sol-Atlas #115 — maritime evidence projection](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/115), [#654 — common maritime evidence projection](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/654) and [#809 — metocean/access context](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/809) own spatial/time context. A planning dataset is not official navigation coverage.
- [Symthaea #11173 — read-only maritime decision support](https://github.com/Luminous-Dynamics/symthaea/issues/11173), [#11185 — operational-mode/authority envelopes](https://github.com/Luminous-Dynamics/symthaea/issues/11185) and [#6260 — MASS/COLREG benchmark profile](https://github.com/Luminous-Dynamics/symthaea/issues/6260) own the corresponding domain-level semantics.
- Reuse existing Platform [OMSI industry profile #154](https://github.com/Luminous-Dynamics/luminous-platform/issues/154), [PNT profile #297](https://github.com/Luminous-Dynamics/luminous-platform/issues/297), [operation outbox/recovery #18](https://github.com/Luminous-Dynamics/luminous-platform/issues/18), [failure semantics #283](https://github.com/Luminous-Dynamics/luminous-platform/issues/283) and OMSI validator/adapter [PR #53](https://github.com/Luminous-Dynamics/luminous-platform/pull/53).

These links identify the current work inventory, not proof that every referenced work item is merged, executed or externally qualified.

## 4. Operator and operation boundary

The v0.1 profile is read-only by default. It may prepare an evidence brief, highlight a conflict, request a review or stage an approval request. It does not issue vessel-control, propulsion, steering, navigation, crane, electrical or other physical commands.

For consequential operations, reuse the existing OMSI contract rather than implementing a second one:

`intent -> authority check -> bounded attempt -> observed effect -> independent postcondition -> receipt / reconciliation`

Required distinctions:
- recommendation != authorization;
- authenticated actor != actor authorized for this subject, action, time and scope;
- attempted operation != completed physical effect;
- command acknowledgement != independently verified postcondition;
- timeout or lost connection != failure and != success;
- unknown outcome -> reconcile the existing operation identity before any retry or new mutation;
- human override/dissent is preserved as an attributable record, not silently overwritten by the model.

The profile should preserve the responsible master/operator and any non-delegable flag, coastal, port, class or statutory authority. It does not infer that a software workflow may replace those roles.

## 5. Maritime standards and regulatory mapping

Every integration must pin exact standard edition, status, vessel/tonnage, service, voyage, flag, class, jurisdiction and intended use. The matrix below is an admission checklist, not a claim that the standards are satisfied.

| Reference | Use in profile | Boundary / claim ceiling |
|---|---|---|
| [IMO MSC.595(111) MASS Code](https://www.imo.org/en/mediacentre/pressbriefings/pages/imo-adopts-mass-code.aspx) and [IMO autonomous shipping overview](https://www.imo.org/en/mediacentre/hottopics/pages/autonomous-shipping.aspx) | Map goals and evidence for relevant autonomous-surface-ship design, approval and operation profiles | Non-mandatory Code effective 1 July 2026. Current scope concerns cargo ships within SOLAS Chapter I; applicability must be checked for vessel type, tonnage, voyage, flag and service. Not a leaderboard score or certificate. |
| [IACS UR E26/E27](https://iacs.org.uk/resolutions/unified-requirements/ur-e) and [IACS applicability update](https://iacs.org.uk/news/iacs-ur-e26-and-e27-press-release) | Lifecycle cyber-resilience and onboard-system/equipment evidence for ships within applicability | Revised requirements apply to new ships contracted for construction on/after 1 July 2024; mandatory vs non-mandatory applicability varies by vessel category, size and service. Pin the applicable revision, contract date and ship/system category. Do not infer compliance from a control checklist or generic cyber score. |
| [IHO S-100 standards in force](https://iho.int/en/s-100-standards-in-force-0) | Versioned marine data/product interoperability (e.g. S-101, S-102, S-104, S-111, S-124, S-129) | Preserve edition, cell/revision, coverage, datum, uncertainty, source time and license. Schema conformance or map display is not navigation approval. |
| [ISO 19030 series](https://www.iso.org/standard/63774.html) | Same-vessel hull/propeller performance-change measurement where the method and vessel are applicable | Trend methodology is not a universal cross-vessel efficiency score. Control environmental/operational confounders. |
| [ITTC](https://www.ittc.info/) / [SIMMAN](https://www.simman2020.kr/) references | Ship manoeuvring, model-test and benchmark method selection | Match scale, geometry, loading, environment, test conditions and evaluator. A simulation result is not proof of safe full-scale handling. |
| [ISO 10303-242:2025 Edition 4 / STEP AP242](https://www.iso.org/standard/84300.html) | Neutral engineering/product-model exchange and semantic preservation | Edition 4 is the published 2025 edition. The Edition 5 committee draft is still under development; do not treat it as normative. A translation receipt or tessellation pass is not proof of a valid manufacturing B-rep, tolerance suitability or buildability. Pin edition and translator/backend. |
| Applicable class, flag, coastal-state, port and shipyard requirements | Resolve exact approval and work-control basis | Only the competent authority/class/yard can determine applicable acceptance requirements; the profile records the mapping and evidence, not the approval itself. |

**Standards watch:** ISO work items or committee drafts for MASS scenarios, ODD/OE taxonomy, verification procedures or software safety must be labelled draft/in-development until formally published and checked in the official catalogue. Do not present a draft as a normative final International Standard.

## 6. Benchmark suite

The programme must use separate task tracks. No universal score should permit an excellent cost or carbon result to compensate for a structural, safety, legal or ecological failure.

### Track A — lifecycle digital-thread fidelity

Measure:
- exact subject/configuration/revision resolution;
- issuer, scope, artifact digest and rights preservation;
- event-time/as-of correctness under late, duplicated, reordered and corrected events;
- lineage completeness and reversibility across transformations;
- loss-aware mapping across admitted standards/vendor profiles;
- ability to retain unknown, unsupported, stale, withdrawn and conflicting states.

Primary metrics: exact-match identity rate; required-field preservation; lineage closure; correction propagation accuracy; duplicate/replay idempotency; stale/unknown false-accept rate; unsupported-field leakage; p50/p95 projection latency and compute cost. Use independent fixtures and independently authored expected outputs.

### Track B — engineering change-impact correctness

For a frozen set of design, material, supplier, software, component, inspection and repair changes, measure precision/recall of:
- affected requirements and analyses;
- installed/ordered components and work packages;
- inspections, tests and commissioning evidence;
- operating modes/envelopes and service dependencies;
- requalification/approval actions that require review.

Report false negatives separately from false positives. A missed dependent qualification is a hard failure in the benchmark; do not hide it in a weighted score.

### Track C — shipyard design-for-manufacture and assembly

Owned by Symtropy's shipbuilding benchmark. Compare source-faithful reproduction before adding realism. Keep these measures distinct:
- man-hours per compensated gross ton (CGT), with direct/indirect/subcontract definitions fixed;
- first-of-class productivity drop-off and learning across sister ships;
- schedule/makespan, weighted tardiness, critical-path delay and schedule nervousness;
- material yield/scrap, rework hours and non-conformance;
- dimensional accuracy and accepted block tolerances;
- pre-erection outfitting completeness;
- transport/lift/access/assembly feasibility within the declared yard/process model;
- inspection access, hold-point completion and disruption recovery.

The public First Marine International global shipbuilding benchmarking report is a historical measurement reference, not a 2026 SOTA table. Use current yard/provider data only with rights and denominator definitions pinned. For public ship-block scheduling corpora, hash and parse original bytes, reconstruct actual sheets/units/objectives, reproduce published baselines and independently verify feasibility before claiming a result.

### Track D — vessel engineering and performance

Owned by Symthaea plus suitable validated domain solvers and test facilities. Separate:
- geometry/manifold integrity and hydrostatic property checks;
- resistance, propulsion, powering and energy;
- seakeeping, manoeuvring and station-keeping where relevant;
- structural response, fatigue, corrosion and material/process qualification;
- model-test/CFD/FEA residuals and uncertainty by observable and operating regime;
- model applicability and out-of-distribution detection;
- lifecycle cost, serviceability and repairability.

For every physical observable, identify a qualified reference: analytical solution, independent solver, model basin/tank test, measured vessel trial or accepted metrology. Publish residuals, uncertainty calibration, coverage and failures by condition. A generic rigid-body simulation kernel must not be presented as CFD, wave physics, structural FEA or a validated ship digital twin.

### Track E — restoration, service and maintenance

Measure actual task completion and accepted return-to-service, not just a predicted remaining useful life:
- fault isolation and diagnosis time;
- isolation/safe-state evidence;
- access/extraction path and tool/lift availability;
- part/revision compatibility and provenance;
- repair/replacement and inspection/retest duration;
- independent post-repair acceptance;
- degraded-service availability and time to restore each declared function;
- repeat failure, rework and unplanned downtime.

Compare fixed equipment, line-replaceable modules, redundancy/degraded-mode patterns and common-family tooling under the same mission and physical access conditions. A modularity label is not evidence of maintainability.

### Track F — people and life aboard

Treat human experience as part of the engineering objective, not cabin styling after hull design. Use task- and location-specific evaluation for sleep, cooking, eating, personal care, circulation, work, learning, maintenance and emergency egress. Measure motion-induced task interruption, task success, exposure duration, fatigue/sickness outcomes, falls/reach/access, workload and handover quality where the study design supports them. Keep occupant/task results separate from vessel-level heave/roll/RMS motion measures.

### Track G — energy and ocean impact

Keep separate and report life-cycle boundaries:
- energy per tonne-nautical-mile or passenger-nautical-mile;
- well-to-wake / lifecycle greenhouse-gas emissions by fuel and electricity assumptions;
- hull/propeller performance degradation and cleaning effect under comparable operating conditions;
- underwater radiated noise by frequency, speed, loading and measurement method;
- biofouling transfer/biosecurity and cleaning waste;
- discharge, waste, materials and end-of-life recovery;
- habitat footprint and construction/dredging impacts with spatial uncertainty.

A lower carbon number does not compensate for a breach of a protected habitat, an unsafe design or an unapproved discharge. Do not publish a net-positive ocean claim without independently validated ecological outcomes.

### Track H — operational and cyber resilience

Measure:
- evidence coverage against the applicable IACS E26/E27 profile;
- configuration/vulnerability drift visibility;
- source and dependency/common-mode completeness;
- incident detection/triage/response/recovery evidence;
- communications, PNT, time and power degradation behavior;
- continuity of declared essential services;
- time to independently verified restoration;
- authority preservation through outage, reconnect, replay and conflicting-event scenarios.

This is an internal evidence/coverage benchmark, not an IACS audit, class statement or statutory certification.

## 7. How to compare with industry and SOTA

### Current shipyard-method evidence: hybrid DES–optimization first

Bordbar et al., *Digitalisation of Shipyard Production Planning: A Review of Simulation, Optimisation, AI, and Digital Twin Methods (2010–2025)*, published in the *Journal of Marine Science and Engineering* on 20 February 2026 ([DOI 10.3390/jmse14040396](https://doi.org/10.3390/jmse14040396)), gives us a useful field-level maturity baseline:

- Discrete-event simulation (DES) is a behavioural core for event timing, stochastic interactions, spatial interference and resource contention.
- Optimization and metaheuristics explore structured multi-objective trade-offs, but can simplify spatial-temporal interactions or assume stable parameters.
- Hybrid DES–optimization (e.g. DES + CP/MILP, evolutionary search or RL) is judged the most operationally credible methodological class in bounded planning contexts because candidate schedules are checked against behavioural feasibility.
- AI/digital-twin research often has richer data integration or synchronisation but much of the evidence remains simulation-based, workshop-bounded or short of longitudinal yard-wide KPI validation.

The review explicitly does **not** establish sustained yard-wide transformation as a settled industry result. Its proposed maturity distinctions are valuable: simulation-only; historical calibration/replay; partial shop-floor or APS/MES integration; and sustained live deployment with demonstrable production KPIs. This profile adopts that evidence ladder. It does not treat "real-time", "digital twin" or "AI-driven" as evidence stages.

**Implication for Symtropy:** the benchmark order should be:
1. Source-faithful deterministic rules/dispatch heuristics.
2. CP/MILP or another exact/bounded optimizer on small instances; expose optimality gaps and timeouts.
3. Strong metaheuristics such as ABC/ALNS with the same hard constraints.
4. Hybrid rolling-horizon optimizer whose candidate schedules are executed/replayed by an independent DES behavioural model.
5. Simulation-trained RL or learned policy, with the same observation horizon, action constraints and execution oracle as the hybrid baseline.
6. Prospective shadow execution and only then controlled yard integration.

Do not give an RL policy credit for future disturbance knowledge, an oracle unavailable to other methods, or a reward that omits rework, transport, inspection, crew/shift and recovery work. The DES evaluator must not import the scheduler's feasibility helpers. Predictive ML may estimate process times or rank search candidates, but a learned forecast is not a schedule feasibility proof.

### Recommended SOTA challenge metrics

Use the following as separate, non-compensable report families; proposed targets remain aspirations until a source-faithful baseline is reproduced.

| Dimension | Primary measure | Stretch target after baseline reproduction |
|---|---|---|
| Hard feasibility | Independent-oracle violations by type; false-feasible acceptance | Zero accepted hard-constraint violations within the declared model |
| Small-instance quality | Exact objective value and proven optimality gap | Exact agreement with oracle on every supported tiny instance |
| Large-instance quality | Makespan, weighted tardiness, cost and utilization by instance | Improve the strongest matched baseline without worsening hard feasibility |
| Schedule robustness | Recovery time, tardiness regret, unserved work after disruption | Lower recovery regret at matched disturbance severity |
| Plan stability | Schedule churn, moved jobs, resource reassignment and operator override burden | Improve recovery without excessive schedule nervousness |
| Physical/model fidelity | Prediction residual and uncertainty by observable; unsupported model-path count | No silent extrapolation; calibrated intervals on held-out regimes |
| Deployment maturity | Evidence stage, integration depth, time coverage and longitudinal KPI duration | Progress from replay to authorized prospective shadow evaluation |

Always publish both the planned schedule and the executed/replayed timeline. Keep predicted completion, simulated completion, MES-reported completion and independently observed completion as separate evidence classes.


A credible comparison is a tournament with frozen rules, not a chart of unrelated paper headline numbers.

For each track:
1. Freeze the vessel/yard/mission class, exact configuration, source/data edition, license and operating envelope.
2. Reproduce a credible incumbent or published baseline first, including its original assumptions and objective.
3. Compare appropriate classical and current methods: parametric/CAD plus conventional optimization, qualified CFD/FEA and tank tests, CP/MILP and dispatch heuristics, ALNS/metaheuristics, and learned/RL methods only where task semantics match.
4. Give every method the same input information, held-out split, hardware, wall-clock, memory, evaluation count and tuning budget. No future disturbance information.
5. Use an independent feasibility/metric oracle that does not import the candidate system's own helpers.
6. Hold out complete vessel families, yards/layouts, operating regimes and disturbance sequences. Avoid random row splits that leak near-identical designs/events.
7. Report every seed/run, infeasible solution, timeout, crash, worst-case result, uncertainty interval, compute cost and model failure.
8. Publish exact code/source/data/configuration digests and a reproducible command.

A stretch target such as a 5–10% improvement on a selected objective can be set only after a matched baseline has been reproduced; it is not meaningful before that. No improvement is accepted if hard constraints, model validity, uncertainty, serviceability, environmental or human outcomes regress beyond their predeclared limits.

### 2026 comparator set to reproduce — not headline targets

| Area | External comparator | What it establishes | Important limitations for our comparison |
|---|---|---|---|
| Generative hull geometry | Anagnostopoulos et al., [*Ship Hull Generation Using GANs & VAE-PCTs: A Comparison Study* (14 September 2026)](https://doi.org/10.3390/jmse14181707) | CAD-to-CAD hull generation compares a convolutional GAN with a VAE + Point Cloud Transformer; it evaluates noise/diversity/distribution metrics and geometry plus an early-stage calm-water resistance estimate over 8,000 generated CAD hulls. | Early design-space exploration using two single-type hull corpora; distribution alignment and geometric plausibility do not prove multi-mission performance, full hydrodynamic validity, fabrication feasibility, structural adequacy, serviceability or out-of-family transfer. Reproduce GAN/VAE-PCT baselines before claiming a generative advantage. |
| Generative hull optimization | Zheng, Wang & Chen, [*The generative marine hull form optimization based on offline data-driven evolutionary algorithm* (15 June 2026)](https://doi.org/10.1016/j.oceaneng.2026.125935) | Uses a learned latent hull manifold with expensive data-driven evolutionary search under a limited CFD evaluation budget; reports a practical hull design case and solver validation. | One case study does not prove robust generalization across vessel classes or build processes. Match CFD solver fidelity, evaluation budget, design constraints, data rights and validation measurements. |
| Shipyard spatial scheduling | Wang et al., [*Spatial-Scheduling Strategy for Irregularly Shaped Blocks Under Disruption in Shipbuilding* (11 July 2026)](https://doi.org/10.1049/cim2.70066) | Combines a mixed-integer model, irregular-block geometry, artificial bee colony initialization, ALNS and recovery under workstation/vehicle disruptions, with real-yard case experiments. | Its objective and yard abstraction differ from other public scheduling corpora. Reconstruct its inputs, metrics and constraint semantics before matching it against our source-faithful Track A; compare the richer disruption/geometry variant separately. |
| Shipboard energy assurance | Kondratyev et al., [*Field-Validated Probabilistic Digital Twin of a Shipboard Microgrid* (29 August 2026)](https://doi.org/10.3390/en19174074) | Uses 360 full-scale 30-minute records across 12 event classes (180 h, 4 s samples). Against its deterministic twin it reports 37.3% lower voltage RMSE, 32.7% lower SOC RMSE, 30.1% lower power RMSE, 13.3% lower energy-not-supplied and missed detections reduced from 12.7% to 7.3%. | One operator and predominantly conventional tankers; supervisory 4 s sampling, no millisecond protection validation, no prospective closed-loop onboard trial, and the combined-fault case retains 3.3% missed detections. Its mean detection delay did not significantly improve against the deterministic twin. This is a specific reference to reproduce, not a universal marine-energy SOTA claim. |

For each comparator, the first replication should be a **faithful reproduction lane**. Only after the published result and evaluator have been reconstructed should we run a distinct generalization/robustness lane: leave out vessel families, environmental regimes, yards, fault types and disruption sequences. The Luminous differentiator is the combined evidence chain from candidate geometry to validated physical analysis, process-specific buildability, as-built lineage, operational service and independently verified restoration—not an unqualified claim that one generator or optimizer is better.

### Evidence promotion

Use the existing OMSI profile admission vocabulary per capability:
- **E0 — Concept:** boundary, owner, decision and assumptions defined.
- **E1 — Executable contract:** schema/validator, synthetic positive and negative controls executed.
- **E2 — Reference validated:** independent oracle/reference reproduction and discrepancy analysis.
- **E3 — External-data validated:** rights-reviewed, held-out external dataset with uncertainty and baseline reporting.
- **E4 — Operationally integrated:** authorized sources/instruments, security/privacy review and accountable operational owner.
- **E5 — Shadow/controlled pilot:** prospective comparison, human override and reconciled outcomes.
- **E6 — Production-assured:** limited declared scope with ongoing security, resilience, recovery and external assurance as required.

These are internal evidence stages. They are not external certification, class approval or legal authorization.

## 8. Industry baseline and build-versus-integrate policy

The current commercial ecosystem is not empty or primitive. Established vendors already provide deep shipbuilding-specific CAD, PLM, production engineering, naval-architecture calculations and fleet data platforms. Luminous must not claim to replace those products on their home ground.

| Industry capability | Established examples and documented capabilities | Luminous strategy |
|---|---|---|
| Shipbuilding CAD and production engineering | [SSI ShipConstructor](https://www.ssi-corporate.com/products/shipconstructor/) covers hull/structure, outfitting, weld management, nesting, NC output, production drawings and build strategy; SSI's [February 2026 release](https://www.ssi-corporate.com/blog-lighthouse/ssi-solution-update-february-2026/) describes ShipConstructor 2026 R4 and ShipbuildingPLM 3.3, while its [May 2026 release](https://www.ssi-corporate.com/blog-lighthouse/ssi-solution-update-may-2026/) announces ShipConstructor 2027 and ShipbuildingPLM 4.0. [CADMATIC Hull](https://www.cadmatic.com/en/products/cadmatic-hull/) covers hull modelling and production information. Its [2026 H1 release](https://cadmatic.com/en/releasehighlights/release-highlights-2026h1/) adds traceable document publication and production-information revision checks; its [2026 H2 release (13 November 2026)](https://cadmatic.com/en/news/new-software-release-2026h2/) adds DSTV profile export, approved-object locking and visible check-in/check-out operations. | Integrate with native project records and neutral product-data exchange. Do not build another hull CAD, nesting, weld or NC tool. Test adapter fidelity and the exact downstream effect of each revision. |
| Shipbuilding PLM / configuration | [SSI ShipbuildingPLM](https://www.ssi-corporate.com/products/shipbuildingplm/) describes an established workflow for vessel/program data, change control, hull effectivity, approvals and continuity from engineering to planning, procurement, production and sustainment. | Mycelix remains the owner of the Luminous configuration/provenance contracts. The profile maps to incumbent PLM identities and change events; it does not require yards to replace their PLM or duplicate its canonical configuration store. |
| Naval architecture / analysis | [NAPA Engineer](https://www.napa.fi/software-and-services/ship-design/software/napa-engineer/) is a published platform available with NAPA Release 2025.2. NAPA describes a unified 3D model across hydrostatics, hull form, layout, stability and hydrodynamics; its visual Node Network supports reusable engineering workflows and optimization scenarios but is labelled Alpha on the product page. | Symthaea orchestrates requirements, candidate comparison and uncertainty; qualified specialist solvers and independent physical references remain responsible for the actual engineering observables. Do not claim a substitute for NAPA stability, strength or class calculations. |
| Vessel data / fleet operations | [Kongsberg Vessel Insight](https://www.kongsbergmaritime.com/products/digital/vessel-insight/) provides vessel-to-cloud infrastructure, edge/IoT connectivity, and interoperability with Modbus, NMEA and OPC UA interfaces; the vendor reports DNV and ABS certifications for its edge/IoT technology. | Integrate owner-authorized, read-only operational data and preserve measurement quality, configuration, source/time semantics and dependency context. Do not replace edge gateways, OEM machinery systems or fleet operation centres. |
| Maritime data ecosystem / assurance services | [DNV Veracity](https://www.dnv.com/veracity/) describes an established data platform and digital marketplace for datasets, APIs, applications and digital services. DNV currently reports more than 50,000 connected vessels and 300,000 users across its partner programme. | Differentiate through interoperable evidence lineage, independent cross-system verification, configuration-change impact, authority/effect separation and transparent claim ceilings—not by claiming a larger data marketplace or acting as a class society. Treat coverage figures as vendor-reported, not independently audited benchmark results. |

These are capabilities described by the vendors and service providers themselves. They establish the industry feature baseline, not independently measured performance. Product marketing claims (including productivity or fleet-coverage numbers) must not be copied into our results as benchmark evidence.

### Where Luminous can be meaningfully different

The opportunity is a **vendor-neutral assurance and decision layer across existing systems**:

1. **Cross-vendor semantic continuity:** carry the exact design/configuration, supplier, inspection, operational and repair identities without pretending every system has one universal data model.
2. **Independent evidence checking:** compare source claims against separately implemented geometry, schedule, physical and measurement references; preserve contradictions and unsupported fields.
3. **Change-to-consequence traceability:** show which installed parts, work packages, inspections, operational profiles and requalification decisions are affected by a revision.
4. **Uncertainty and applicability:** show exactly what a solver, sensor, simulation and source can support—and where it cannot.
5. **Authority and recovery:** distinguish recommendation, human approval, attempted effect, measured outcome and independently accepted postcondition.
6. **Lifecycle and human/ecological outcomes:** make maintainability, safe return-to-service, crew task performance, underwater noise, biosecurity and lifecycle impacts first-class, separately reported constraints.

The first commercial integration test should be read-only and source-faithful: ingest a licensed sample of an incumbent CAD/PLM export, preserve part/configuration identity and revisions, compare a change against its source BOM/work package and inspection lineage, and produce a discrepancy report. Only a yard and data owner can authorize real integration or operational use.

### Industry comparison gates

- Do not compare Luminous with a CAD/PLM product on native modelling depth or production drawing maturity until that capability exists and has been independently tested.
- Compare data interoperability using exact fixture exports, source digests, required-field retention, round-trip geometry/metadata tolerances and revision/effectivity preservation.
- Compare change propagation using independently authored ground truth, with safety-relevant false negatives reported as hard failures.
- Compare operational data platforms only on matching sensor coverage, source quality, time resolution, vessel type, edge conditions, cybersecurity profile and service outcome.
- Compare class/flag approval only through the competent independent authority; Luminous can prepare traceable evidence but cannot self-award approval.

## 9. Initial execution sequence

1. Review this profile with Symthaea, Symtropy, Mycelix and Sol-Atlas owners; correct semantic ownership before implementing adapters.
2. Freeze the design-only vectors in [maritime-operations-conformance-vectors-0.1.json](./maritime-operations-conformance-vectors-0.1.json); integrate into existing OMSI validator tooling, not a new validator.
3. Admit one public ship-block scheduling dataset: download original bytes; verify provider terms; hash files; inspect sheets/units/objectives; create a manifest; reproduce one published baseline; independently verify schedules. Until this succeeds, no numerical SOTA result exists.
4. Build one small, synthetic digital-thread case spanning a vessel design revision, a substituted serialized part, yard work/inspection, as-built configuration, commissioning, a maintenance event and impact/requalification review.
5. Add one read-only spatial/metocean artifact projection from Sol-Atlas and one deterministic Symtropy run receipt. Keep unsupported physics explicitly unsupported.
6. Compare platform/event/evidence projections with native source artifacts; then run negative controls for stale, conflicting, out-of-scope and unauthorized conditions.
7. Only after E1/E2 should an authorized shadow workflow or physical yard/vessel pilot be discussed.

### Initial demonstrator selection

Use a civilian modular workboat / research craft as the first end-to-end demonstrator, not a large commercial MASS vessel. This class is suitable for iterating on configuration traceability, fabrication, service access, human habitability and low-risk test instrumentation, subject to competent naval-architecture and regulatory review. The vessel is a benchmark target, not an approved design.

## 10. Current evidence ceiling and non-claims

As of this draft:
- the cross-stack owner map and linked maritime work items exist;
- Symtropy PR #2052 describes a limited synthetic verifier scaffold and explicitly says exact-head tests have not been run in its authoring environment;
- Symtropy PR #4204 records ship-block data-source metadata and explicitly says raw workbook bytes/hashes and numerical reproduction were not completed;
- Symthaea's hull work includes candidate geometry/section/displacement-surface stages; a waterplane-capped mesh or STEP translation receipt is not a complete fabrication-ready hull B-rep or shipyard buildability proof;
- Mycelix's #6154 is a lifecycle contract work item, not a claim that a regulated vessel product-passport system is deployed;
- Sol-Atlas #115 is a source-qualified spatial projection design, not navigational authority;
- this OMSI profile and its vectors are documentation/design artifacts until validator execution and independent review.

No claim is made here of: SOTA performance; validated CFD/FEA; physical shipyard performance; full-scale vessel trials; energy/noise/ecological improvement; MASS, IACS, class or flag-state certification; vendor interoperability; production readiness; or autonomous vessel safety.

## References

- IMO MASS Code adoption and scope: https://www.imo.org/en/mediacentre/pressbriefings/pages/imo-adopts-mass-code.aspx ; https://www.imo.org/en/mediacentre/hottopics/pages/autonomous-shipping.aspx
- IACS UR E26/E27: https://iacs.org.uk/resolutions/unified-requirements/ur-e
- IHO S-100 Standards in Force: https://iho.int/en/s-100-standards-in-force-0
- ISO 19030-1:2016 (general principles, current following 2022 review): https://www.iso.org/standard/63774.html; ISO 19030-2:2016 (default method): https://www.iso.org/standard/63775.html; ISO 19030-3:2016 (alternative methods): https://www.iso.org/standard/63776.html
- ISO 10303-242:2025 Edition 4: https://www.iso.org/standard/84300.html; Edition 5 committee draft (under development, not normative): https://www.iso.org/cms/live/live/en/sites/isoorg/contents/data/standard/09/32/93277.html?browse=tc
- ITTC: https://www.ittc.info/
- SIMMAN: https://www.simman2020.kr/
- 2026 hull-generation comparator: https://doi.org/10.3390/jmse14181707
- 2026 offline data-driven hull optimization comparator: https://doi.org/10.1016/j.oceaneng.2026.125935
- 2026 irregular ship-block scheduling/disruption comparator: https://doi.org/10.1049/cim2.70066
- 2026 field-validated shipboard microgrid comparator: https://doi.org/10.3390/en19174074
- Historical First Marine International / US Navy / NSRP shipbuilding benchmarking report (2007): https://www.nsrp.org/wp-content/uploads/2017/05/Part2_FMI_Benchmarking_Study.pdf
