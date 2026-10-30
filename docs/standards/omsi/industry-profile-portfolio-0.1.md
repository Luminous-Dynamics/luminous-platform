# OMSI Industry Profile Portfolio and Assurance Protocol 0.1

**Status:** proposed portfolio policy; not an adopted industry standard  
**Date:** 2026-10-30  
**Issue:** [OMSI-INDUSTRY-001](https://github.com/Luminous-Dynamics/luminous-platform/issues/154)  
**Claim ceiling:** this document defines profile admission and evaluation rules. It does not certify any profile, solver, adapter, product, organization or deployment.

## 1. Executive decision

Use **OMSI as the cross-industry evidence/interchange and consequential-operation framework**. Do not create a parallel Industry Engineering Framework schema or a universal industrial ontology.

The common layer standardizes how source identity, scope, time, uncertainty, mapping decisions, evidence, provenance, conformance and authority boundaries are carried. It does not make a water observation, clinical observation, work order, assay, shipment, machine alarm and production event semantically interchangeable.

The existing architecture proposal already makes the important separation:

1. **Interchange/evidence envelope** — portable source claims and derived evidence.
2. **OMSI operation contract** — intent, authority, attempt, effect, recovery and postcondition verification when a consequential operation is requested.
3. **Industry profile** — domain entities, units, relationships, rules, hazards, assurance requirements and claim ceilings.
4. **Vendor/source adapter** — version-pinned extraction and loss-aware mapping.
5. **Transport binding** — files, APIs, event streams, OPC UA or other transport without changing meaning.

Reference: [OMSI cross-industry extension architecture, PR #52](https://github.com/Luminous-Dynamics/luminous-platform/pull/52).

## 2. Why profile-first beats a universal industrial application

A generic object model such as Asset, WorkItem, Status, Measurement or Evidence can hide distinctions that matter:

- A work order reported complete is not independent proof of completed work.
- A water service is a time-varying hydraulic flow with pressure, storage, quality and demand constraints; it is not a generic capacity number.
- A crop observation is not a yield measurement, and a satellite-derived water-productivity estimate is not a plot-level soil measurement.
- An ore occurrence is not a sample assay; an assay is not a reserve; an element present is not necessarily recoverable product.
- A radio coverage estimate is not network reachability, capacity adequacy, observed QoS or delivered service.
- A machine condition estimate is not an authorized maintenance action or a product-quality release.

The common layer should standardize how meaning, uncertainty and provenance travel. Domain profiles must retain the physics, units, temporal semantics, authority and acceptance rules specific to each industry.

## 3. Portfolio register — current position

This register is a repository-level work inventory, not an independent qualification report. Links identify work and design artefacts; an open PR, issue, module, test count or passing unit test does not establish external validity or production readiness.

| Profile | Current owner and work | Current evidence ceiling | Next material qualification gap |
|---|---|---|---|
| Manufacturing asset operations / quality evidence | Luminous Platform OMSI: [cross-industry architecture PR #52](https://github.com/Luminous-Dynamics/luminous-platform/pull/52), [Rust validator/adapter PR #53](https://github.com/Luminous-Dynamics/luminous-platform/pull/53) | Synthetic profile, validator and mapping/replay/provenance tooling are proposed in open stacked PRs. No vendor interoperability or physical quality authority is established. | Exact-head build/CI, independent implementation, authorized real source adapter, measurement-system and plant validation. |
| Water / water-energy / municipal lifelines | Sol-Atlas: [spatial-economic resilience PR #116](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/116), [EPANET hydraulic localization PR #112](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/112), issues [#139](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/139), [#144](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/144), [#147](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/147). Mycelix utility KPI semantic drift PR [#5435](https://github.com/Luminous-Dynamics/mycelix/pull/5435). | Extensive proposed Rust evaluators, synthetic scenarios and source-attributed public observations. PR #116 remains a draft/open dependency in this portfolio snapshot; EPANET and source comparisons still require exact-head execution and matched validation. | Pin solver/input versions; qualify hydraulic residuals against EPANET; validate topology and field observations; keep service-cell reachability separate from delivered pressure/quality/continuity. |
| Agriculture / food / soil-water | Symthaea [CropFoundry PR #8046](https://github.com/Luminous-Dynamics/symthaea/pull/8046); Symtropy [soil-water kernel PR #2848](https://github.com/Luminous-Dynamics/symtropy/pull/2848); Mycelix [Terra Preta protocol PR #5513](https://github.com/Luminous-Dynamics/mycelix/pull/5513) and soil transfer work [#5534](https://github.com/Luminous-Dynamics/mycelix/issues/5534). | CropFoundry proposes a multi-dimension benchmark and explicit source permissions; the Symtropy PR is an accounting kernel, not a Richards-equation, crop or soil-efficacy model; Mycelix's protocol/transfer tasks remain research work. | Resolve source rights and quarantines; compare appropriate model lanes; site/season-held-out evaluation; laboratory metrology; replicated field trials and actual water/yield/economic outcomes. |
| Mining / geometallurgy | Symthaea [ORE-GEO-001 #7995](https://github.com/Luminous-Dynamics/symthaea/issues/7995); Symtropy [ORE-FEED-BLEND-001 #2002](https://github.com/Luminous-Dynamics/symtropy/issues/2002). | Domain model and bounded synthetic blend-oracle specifications; no general validated ore-to-product digital twin. | Sample- and lot-specific assay/mineralogy/QC; comminution and liberation measurements; validated process recovery; tailings/residue and water/energy closure; plant reconciliation. |
| Telecommunications resilience | Sol-Atlas [CONNECT-RES-001 #323](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/323), design [PR #324](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/324), executable follow-up [#325](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/325). | Design specification plus planned topology/power/service adapter; no RF predictor, operator integration, QoS measurement or field claim. | Reuse Sol-Atlas service network, N-k and service-floor components; independent site-power audit; actual authorized site/backhaul and service data; later RF/RAN comparison. |
| Built environment / construction | Sol-Atlas construction and asset work, including [construction-systems issue #146](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/146). | Research and design tasks; not a construction BIM, structural approval or whole-life asset product. | IFC/BIM exchange, applicable building codes and engineering sign-off, measured asset performance, commissioning and whole-life maintenance data. |
| Logistics / ports / cold chain | Mycelix transport and supply-chain domain foundations; related portfolio issue [#3095](https://github.com/Luminous-Dynamics/mycelix/issues/3095). | Domain coordination and traceability foundations; not a qualified dispatch, port or cold-chain operations suite. | WMS/TMS/port data interoperability, temperature-chain evidence, capacity/time-window constraints, exception handling and route/stockout field outcomes. |
| Chemical/process industries | Symtropy process and materials research; Symthaea process/industrial work. | Research-stage domain footholds; no general process engineering qualification. | Thermodynamics, reaction kinetics, mass/energy closure, process safety, validated flowsheets and operator-authorized HAZOP/LOPA boundaries. |
| Healthcare / laboratory / pharmaceuticals | Mycelix health research and Symthaea biology/clinical-adjacent research. | Research and data/identity foundations; no clinical diagnostic or regulated production authority. | Regulatory pathway, clinical/lab validation, privacy/security controls, quality systems, independent clinical authority and prospective evidence. |

### Current portfolio priority

Run the following three flagship pilots as different test families, not as three versions of the same product:

1. **Connectivity resilience:** new vertical, tests power/backhaul/topology dependencies and essential-service floors.
2. **Water–food–energy:** strongest cross-domain reuse, tests physical-model composition, irrigation/soil/crop evidence and resource trade-offs.
3. **Industrial maintenance:** tests partial observability, maintenance decisions, resource-constrained repair and production consequences.

Keep mining and construction active as research tracks, but do not expand them into broad product claims before their domain evidence is ready.

## 4. Industry profile admission contract

No vertical should enter the portfolio as an implementation project until it supplies a versioned profile brief with the following fields.

| Required section | Required questions | Rejection / defer condition |
|---|---|---|
| Decision boundary | What exact decision or workflow is improved? Who is the decision owner? | Broad ambition with no measurable decision or accountable operator. |
| Domain model | What are the domain entities, relations, invariants and lifecycle states? | Generic status fields erase material domain meaning. |
| Physics / authoritative model | What equations, solver, standard, analytical result or domain authority is authoritative for each claim? | A general simulator is presented as validation for a domain it does not model. |
| Units and metrology | What quantity kind, unit, reference frame, calibration, uncertainty and sample basis apply? | Incompatible/unknown units or measurement context silently normalized. |
| Data and rights | What exact source, version, byte hashes, permissions, licensing, spatial/temporal resolution, privacy and retention rules apply? | Downloadable data is assumed to be trainable, redistributable or production-authorized. |
| Evidence and provenance | Can every input and derived result be traced to its origin, method and exact bytes/version? | Digest, signature or ledger entry is treated as proof of source truth. |
| Uncertainty and missingness | How are bounds, unknowns, stale values, outliers, conflicts and abstentions represented? | Missing/unknown values become zeros, successes or positive availability claims. |
| Operator workflow | How is a recommendation reviewed, approved, executed, inspected and reconciled? | No human escalation, correction, dispute or recovery path for consequential work. |
| Safety and authority | Which actions are read-only, advisory, approval-gated or prohibited? What are the hard service/safety floors? | AI model, adapter or evidence ledger implicitly gains physical control or regulated authority. |
| Privacy and security | What data minimization, tenant scope, purpose limitation, access controls and threat model apply? | Subscriber/patient/employee secrets or production credentials enter a synthetic benchmark. |
| Validation plan | What analytical/reference cases, negative controls, held-out sources, independent verifier and field protocol exist? | Random row split, self-verification only, or unit tests promoted to external validation. |
| Commercial outcome | Which customer/beneficiary outcome and cost denominator changes? | Value proposition depends only on model score, feature count or unmeasured savings. |

## 5. Evidence promotion stages

Use one portfolio vocabulary, while retaining industry-specific qualification requirements.

| Stage | Meaning | Minimum evidence to claim stage |
|---|---|---|
| E0 — Concept | Problem and architecture hypothesis | Scope, decision, owner and assumptions written down. |
| E1 — Executable contract | Versioned schemas, semantic rules and synthetic/analytical fixtures | Validators execute; known answers and negative controls are exercised; exact inputs and evaluator identity recorded. |
| E2 — Reference validated | Compared with an appropriate analytical result or independently maintained reference model | Matched units, boundary conditions, parameterization, version pins, residuals/tolerances, discrepancy analysis and independent reproduction. |
| E3 — External-data validated | Tested on a genuinely held-out external dataset | Rights reviewed, source pinned, grouping/time/region holdout justified, uncertainty/calibration and all denominators reported. |
| E4 — Operationally integrated | Authorized source systems or instruments integrated | Data contracts, security/privacy review, source freshness and quality, failure/recovery semantics, and operational owner accepted. |
| E5 — Shadow/controlled pilot | Compared prospectively with real operator decisions/outcomes without unsafe autonomous control | Protocol frozen, baseline collected, intervention exposure documented, human override and incident response proven, outcome denominator reconciled. |
| E6 — Production-assured | Limited operational scope with accountable owner and continuing assurance | Security, resilience, recovery, human authority, service commitments, monitoring, drift, incident handling, rollback and external/regulatory duties reviewed for that deployment. |

These are internal portfolio stages, not external certifications. They are not necessarily a simple linear maturity ladder: a profile can have a validated physical submodel and an unqualified workflow, or a live data adapter and an unvalidated prediction model. Record stage per capability and claim, not only per industry label.

Promotion is claim-specific. A battery accounting kernel can be E2 for arithmetic and still E0 for battery lifetime prediction. A model may be E3 for one crop/site/season and E0 for another.

## 6. Benchmark protocol: how we compare fairly

### 6.1 Freeze the task before choosing the model

Each benchmark preregistration must declare:
- target decision and output;
- evaluation unit and denominator;
- evidence/data releases, source licensing and exclusions;
- operating conditions and applicability domain;
- time, site, machine, topology, lot or patient grouping;
- train/calibration/test partitions, with leakage analysis;
- metrics, uncertainty method and acceptance thresholds;
- model/solver versions, hardware, compute, energy and tuning budgets;
- baselines, ablations, negative controls and independent verifier;
- claim ceiling and protocol change policy.

A performance number without these fields is not a comparable benchmark result.

### 6.2 Use a baseline ladder

Every profile should include as applicable:

1. **Trivial baseline:** persistence, zero/mean, no-intervention or other simplest legitimate rule.
2. **Current-practice baseline:** the actual workflow/policy currently used, where access and measurement are authorized.
3. **Established specialist baseline:** the most relevant reference solver or widely used domain model.
4. **Simple transparent policy:** deterministic engineering heuristic or rule-based control.
5. **Candidate model:** the Symthaea/Symtropy/Sol-Atlas method under test.
6. **Bounded exact oracle:** only for small cases where the entire feasible state/action space is enumerable; label it non-deployable.
7. **Human expert baseline:** where a real operator decision is the relevant comparator, using a frozen protocol and appropriate blinding.

No straw-man baseline. No comparison to a different task. An exact oracle is an upper-bound comparator, not a fair deployed competitor.

### 6.3 Do not create one global industry score

Report separate metric families:

- **Physical/model fidelity:** analytical error, mass/energy balance residuals, solver convergence, sensitivity and out-of-domain behavior.
- **Predictive quality:** MAE/RMSE or task-specific metrics, precision/recall, lead time and calibrated uncertainty.
- **Decision quality:** regret, constraint violations, false alarms, missed actionable events, abstention and human override.
- **Operational outcome:** good output, delivered service, outages, spoilage, downtime, water/energy per accepted unit and recovery.
- **Economics:** lifecycle cost, resource cost, avoided loss and uncertainty in monetary attribution.
- **Assurance:** adversarial controls, evidence completeness, reproducibility, independent verification, security and recovery.
- **Computational efficiency:** end-to-end latency, throughput, memory, hardware, compute energy and cost per evaluated decision.

Hard safety, legal, product-quality and essential-service constraints cannot be offset by higher aggregate efficiency.

### 6.4 Rules for an honest SOTA claim

A result may be described as outperforming a named baseline only if:
- the task and output are identical;
- the same data release and permissible information are used;
- train/test separation is at the correct independent unit;
- tuning, compute and resource budgets are comparable;
- metrics and uncertainty are predeclared;
- all baselines are independently reproduced or their exact source/results are cited;
- negative controls and adverse regimes are reported;
- the exact executable and input identities are published;
- all material regressions and non-comparable cases remain visible.

Use a not-comparable result if inputs, task definitions, data populations, physics assumptions or metrics cannot be reconciled. Never normalize unlike scores into a single winner.

## 7. Profile-specific benchmark lanes and current references

### 7.1 Water and water-energy

**Reference lane:** EPA EPANET 2.2 for the hydraulic cases it supports; use Water Network Tool for Resilience (WNTR) as a separate analysis tool when appropriate. A graph max-flow model is not a substitute for hydraulic head, pressure, pump curves or water quality.

**Task lanes:**
- hydraulics: node pressure/head and link flow under matched topology, demand, controls and boundary conditions;
- leakage: binary detection and pipe/node localization as separate tasks;
- generalization: topology-held-out and operating-condition-held-out cases;
- water-energy: matched pump operating point, pump-terminal power, specific energy and hydraulic constraints;
- service continuity: community floors and distributional deficits under explicit joint failures.

**Metrics:** pressure/flow error and residuals, pipe/node top-1/top-3 localization, graph/geodesic localization distance, false-alarm burden, uncertainty coverage, unmet demand volume, time-to-recovery bounds, per-community essential-service compliance and energy per delivered unit.

Do not compare a Net3 synthetic test result with a different field sensor modality as one leaderboard. Keep hydraulic solver parity, leak detection, field localization and service continuity separate.

### 7.2 Agriculture, food and soil-water

**Reference lane:** FAO AquaCrop **7.3**, released 30 July 2026 with a Python implementation and refinements to soil hydraulic-property estimation and simulation; FAO WaPOR **v3 API** for remote-sensing-derived water-productivity data where the spatial level and product are suitable. WaPOR is contextual observation, not farm-scale ground truth. AquaCrop is a crop-water response model for herbaceous crops; it is not a general soil physics, orchard, livestock or whole-farm solver.

**Task lanes:**
- regional screening and water productivity;
- crop-water/yield response with matched AquaCrop inputs;
- soil-water accounting and later constitutive/flow-model qualification;
- irrigation-policy decision value under fixed water caps;
- site/season-held-out transfer;
- field trials with preregistered design and actual yield/water/quality measures.

**Metrics:** yield error by crop/site/season, irrigation water productivity, water productivity per unit output, unmet water demand, plant stress proxy with declared limitations, input/resource use, post-harvest retention, policy regret, uncertainty coverage and field-trial power/replication adequacy.

Never treat a synthetic soil ledger, regional satellite product or one season's simulation as proof of agronomic efficacy.

### 7.3 Manufacturing and condition-based maintenance

**Reference lanes:** NIST SimPROCESD is a discrete-event manufacturing/maintenance simulator for asynchronous finite-buffer lines with degradation and maintenance; its NIST page lists last update 15 August 2024, so pin source/version and assess fitness rather than calling it the latest SOTA. NASA's Prognostics Center of Excellence data repository provides donated prognostics datasets, including run-to-failure data; NASA explicitly puts use at the user's risk. Dataset-specific terms and provenance must be checked.

**Task lanes:**
- degradation/prognostic model accuracy and calibrated remaining-life intervals;
- condition-monitoring detection under noise, missingness and regime shift;
- maintenance policy value under constrained crews/spares;
- discrete-event production effect under finite buffers;
- quality/measurement interoperability through OMSI.

**Metrics:** false alarms per asset-time, missed actionable failures, warning lead time, remaining-useful-life interval coverage, maintenance crew utilization, repair queue delay, downtime, good units, scrap/rework, WIP/flow-time tails, total cost and decision regret.

A model with better RMSE may still produce a worse policy. Compare maintenance policy outcomes, not just sensor-model metrics.

### 7.4 Mining and geometallurgy

**Reference lane:** no single universal public benchmark can validate the entire deposit-to-product chain. Split geology, sampling, assay, mineralogy, comminution, liberation, blending, beneficiation, recovery, residue and economics into separate qualification lanes.

**Metrics:** assay repeatability and QA/QC, sample representativeness, mass/grade balance, mineralogical and liberation error, comminution energy, water/reagent consumption, recovery and product-quality bounds, tailings/residue closure, and lot-to-product traceability.

An occurrence map is not a reserve; an assay is not plant recovery; a synthetic feed-blend oracle is not a qualified plant optimization.

### 7.5 Telecommunications resilience

**Topology and energy lane:** Sol-Atlas service-network/stress/equity evaluators reused through CONNECT-RES-002. Evaluate declared site/backhaul routes, power dependencies, backup energy and essential service floors.

**Radio lane:** ITU-R P.1812-8 (09/2025) is in force for path-specific terrestrial propagation from 30 MHz to 6 GHz. 5G-LENA **v5.1** (6 August 2026) is compatible with ns-3.48; release notes describe v5.2 as under development. Pin v5.1/ns-3.48 for reproducible reference runs rather than tracking a moving development branch. Hardware-in-the-loop is a later, distinct Colosseum lane.

**Metrics:** separately report RF prediction error, coverage by declared threshold, reachability, capacity, QoS, path diversity, site autonomy, unserved demand, outage/recovery bounds, service-floor compliance and capital cost. A topology resilience score is not RF coverage or packet-level 5G performance.

### 7.6 Buildings, logistics, chemical/process and healthcare

Keep these at profile proposal until a decision target, authoritative models, reference data, responsible operator, privacy and safety requirements are identified.

Potential standard/reference candidates must be scoped and version-pinned before implementation. Do not assume an IFC file validates structural safety, a shipment scan proves cold-chain integrity, a flowsheet validates reaction kinetics or a signed clinical record proves a diagnosis.

## 8. Cross-industry assurance and interoperability references

### Digital twin VVUQ

NISTIR 8620 (21 July 2026) summarizes NIST manufacturing digital-twin workshops and identifies interoperability, verification/validation/uncertainty quantification, cybersecurity and workforce readiness as ongoing challenges.

ISO/AWI 23247-8 was approved as a work item on 23 October 2026 and is **under development**. Its stated scope is digital-twin VVUQ throughout the lifecycle. It is not a published International Standard and must not be cited as a current compliance requirement.

References:
- NISTIR 8620: https://www.nist.gov/publications/digital-twins-workshops-summary-report
- ISO/AWI 23247-8 status: https://committee.iso.org/standard/94858.html

### Data and industrial exchange

Use established standards through narrow, version-pinned profile subsets rather than replacing them:
- ISA-95 / IEC 62264 for relevant enterprise/manufacturing operations boundaries;
- ISO 55000:2024 terminology and asset-management principles;
- OPC UA information models or data access where actually used (do not enable command/control in a read-only profile);
- UCUM unit codes where appropriate, while separately defining quantity kind and domain constraints;
- applicable quality, provenance and measurement standards by profile.

The OMSI manufacturing profile and interoperability target matrix are already the starting points. This document does not declare conformance to any of those standards.

## 9. Reusable implementation path

Each industry should reuse the same process, not necessarily the same domain evaluator:

1. **Register the profile:** owner, decision, scope, claim ceiling and admission checklist.
2. **Inventory sources:** exact versions, byte hashes, permissions, licensing, spatial/temporal resolution, privacy and known omissions.
3. **Define domain semantics:** entities, relationships, units, time basis, invariants, hard constraints and unknown states.
4. **Build the domain reference:** use a suitable external solver or bounded analytical model; do not make Symtropy the reference for itself.
5. **Add adapters:** use OMSI's source identity, snapshot, loss-aware mapping, provenance and independent verification patterns.
6. **Add benchmark lanes:** known answers, negative controls, baselines, held-out data, uncertainty, operational policy outcomes and cost.
7. **Separate candidate selection and checking:** the independent verifier reconstructs the result without calling the predictor/optimizer.
8. **Integrate read-only first:** authorized data, least privilege, no control path.
9. **Run shadow evaluation:** compare to current operator decisions and real outcomes.
10. **Promote by claim:** only the exact validated capability advances; other features retain their lower evidence stage.

The same source bytes, schema, mapping, evaluator, baseline, environment and output report should produce a replayable assurance bundle. Cryptographic digests bind byte identity; signatures bind an asserted signer; neither alone proves source truth or physical reality.

## 10. Productization rule

A vertical should earn its place in the commercial portfolio by solving one narrow high-value decision before expanding its module catalogue.

For each flagship:
- name one initial customer/operator;
- establish a baseline workflow and outcome measurement;
- identify a paid or economically material decision;
- limit initial deployment to read-only planning or shadow evaluation;
- demonstrate a measurable improvement without weakening critical constraints;
- capture operating cost, setup/integration time, required data and human workload;
- publish what remains unsupported.

The initial product candidates are:
- telecom site/backhaul/power resilience planning;
- water–food–energy resource and irrigation planning;
- maintenance policy and industrial production continuity.

## 11. Profile change, retirement and supersession

Every profile version has an immutable ID and source pins. Changes to units, semantics, safety limits, evidence thresholds, source mapping or claim ceilings require a new version and migration note. A profile may be deprecated when a better domain owner/reference exists, data rights fail, evidence is irreproducible, or a competing profile duplicates its scope.

Never silently rewrite past benchmark outputs. Preserve the exact profile, source, evaluator and result identifiers used at the time.

## 12. Open decisions

1. Which initial operator/customer will supply authorized real data for each flagship?
2. Which external model releases can be legally pinned and redistributed as reproducible test dependencies versus referenced as external executables?
3. Which evidence claims require third-party inspection, accredited laboratory results or statutory sign-off?
4. Which profile should be first to progress from E2 reference validation to E3 held-out external validation?
5. Which cross-project evidence bundle format should be the first interoperable OMSI implementation?

## 13. Decision

Adopt the OMSI profile-first direction as the working architecture proposal. Continue the manufacturing validator as the first implementation of the shared evidence/adapter patterns. Use the three flagship pilots to test profile reuse, not to create three competing core frameworks.

**No industry, solver, adapter, dataset or product is promoted by this portfolio document.** All stage claims must be supported by the exact artifact, independent verification and evidence appropriate to that claim.
