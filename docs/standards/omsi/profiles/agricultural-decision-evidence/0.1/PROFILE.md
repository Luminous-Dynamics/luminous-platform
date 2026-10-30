# OMSI Agricultural Decision Evidence Profile 0.1 — incubation

**Status:** synthetic read-only conformance proposal; not adopted or certified  
**Date:** 2026-10-30  
**Implementation status:** profile and fixture contract only; no vendor connector, farmer deployment, agronomic recommendation authority or field validation is claimed.  
**Parent:** [OMSI-AGRI-001 #156](https://github.com/Luminous-Dynamics/luminous-platform/issues/156)

## 1. Purpose

Test whether the OMSI interchange/evidence envelope can carry farmer and agriculture records without erasing agronomic meaning or collapsing advice, farmer choice, physical action, observation and measured outcome into one generic workflow status.

This profile is read-only. It does not create farm/land authority, a farmer identity registry, soil/crop model, irrigation controller, service marketplace, offline journal, payment authority or training-data permission system.

The profile should project domain-owned records, preserve source and model identities, expose mapping loss and uncertainty, and explicitly withhold authority to act.

## 2. Reuse and ownership

| Concern | Existing owner/reference | OMSI profile boundary |
|---|---|---|
| Crop/irrigation experiments | Symthaea CropFoundry [PR #8046](https://github.com/Luminous-Dynamics/symthaea/pull/8046) | Preserve the exact policy/model/source version and synthetic-vs-observed status; do not recreate irrigation policies. |
| Farmer-controlled advice | Symthaea [AGRI-ADVICE-001 #8027](https://github.com/Luminous-Dynamics/symthaea/issues/8027) | Carry premises, applicability, prediction interval, explanation and farmer choice; do not decide on behalf of the farmer. |
| Farmer workflow and outcomes | Mycelix [FARMER-UX-001 #5194](https://github.com/Luminous-Dynamics/mycelix/issues/5194) | Preserve request/response/action/acceptance lifecycle references; do not create a second farmer-service ledger. |
| Offline recovery | Mycelix [FARMER-UX-004 #5202](https://github.com/Luminous-Dynamics/mycelix/issues/5202) | Preserve local operation identity and OutcomeUnknown semantics; do not implement a new client journal here. |
| Soil water arithmetic | Symtropy [SOIL-TWIN-001 PR #2848](https://github.com/Luminous-Dynamics/symtropy/pull/2848) | Preserve units and supplied-flux accounting status; do not present the kernel as a soil-flow, crop or amendment-efficacy model. |
| Soil amendment and hydrology trials | Mycelix [Terra Preta PR #5513](https://github.com/Luminous-Dynamics/mycelix/pull/5513), [NEON transfer issue #5534](https://github.com/Luminous-Dynamics/mycelix/issues/5534) | Carry sample/study/site/sensor and proxy-label provenance; do not infer efficacy from a protocol or proxy outcome. |
| Water and spatial infrastructure | Sol-Atlas [water reference PR #112](https://github.com/Luminous-Dynamics/sol-atlas-leptos/pull/112), [water-energy work #144](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/144) | Carry source water/power context and solver references; do not infer hydraulics from an aggregate graph. |

The common OMSI envelope, mapping-loss accounting, provenance verifier and replay rules are shared infrastructure. This profile defines only agriculture/farmer-specific semantics and conformance expectations.

## 3. Domain boundaries that must remain distinct

1. **Source observation** — a statement from a named sensor, laboratory, farmer, field record or public dataset. It is not automatically accurate or representative.
2. **Forecast** — a time-bounded prediction about future weather, water availability or other conditions. It is not an observation of what occurred.
3. **Modelled output** — a result produced by an identified model, version, configuration and input set. It is not a measured crop, soil or farm outcome.
4. **Derived accounting** — arithmetic that transforms supplied quantities (for example, a soil-water balance). It does not validate the supplied fluxes.
5. **Advice** — a candidate option with premises, applicability limits, constraints and uncertainty. It is not an authorized operation.
6. **Farmer decision** — accepted, modified, deferred, declined or referred. It is not proof that the recommended operation occurred or that the outcome improved.
7. **Action** — a separately authorized and recorded physical or service operation. A local draft, submitted request, provider acknowledgement, completed service and farmer acceptance are distinct states.
8. **Outcome** — a later measurement or observation with its own method, time, sample, source and uncertainty. It is not automatically attributable to prior advice or action.
9. **Causal treatment effect** — requires an appropriate prospective or otherwise defensible study design. It must never be inferred merely from advice followed by a later outcome.

No common status field may collapse these meanings.

## 4. Minimum profile fields

A domain record projection includes, where available:

### 4.1 Source identity and scope
- source system, source instance namespace, source contract/release pin, object type, stable external ID and source revision/snapshot token;
- independently configured trusted scope binding and tenant/workspace;
- source-side farm, farm unit, plot, field, crop, lot or asset references separately namespaced and kept distinct from authorization evidence;
- export, extraction and ingestion times separately from event, observation, valid/effective and source-modified times.

A farm or plot identifier supplied by a source is not proof of ownership, consent, tenant authorization or permission to export data.

### 4.2 Evidence and metrology
- evidence kind: measured, reported, forecast, modelled, inferred, derived, simulated, unknown, stale, conflicting or retracted;
- quantity kind, exact decimal lexical value, unit system/code, measurement basis, sample/replicate and spatial/depth/area reference frame where relevant;
- measurement method, sensor/instrument identity, calibration reference and validity claim when supplied;
- uncertainty and its stated meaning/coverage convention; unknown uncertainty remains unknown;
- source quality/QC flags and limitations without upgrading them to independent validation.

Do not treat volumetric soil moisture, gravimetric moisture, water depth, volume, flow rate, irrigation application, rainfall, evapotranspiration, yield, edible mass and water productivity as interchangeable because values look numerically similar. A unit string alone does not specify soil depth, dry/fresh mass basis, area denominator or measurement protocol.

### 4.3 Model and advice provenance
For a modelled output or recommendation, preserve:
- model name, exact version/build, profile/configuration, parameters and declared applicability domain;
- exact premise/source/evidence references and their versions;
- forecast time horizon, valid time and input as-of time;
- prediction interval/uncertainty, known exclusions and out-of-domain state;
- candidate action and hard constraints separately from a recommendation score.

An output from CropFoundry synthetic irrigation-policy testing remains synthetic. AquaCrop 7.3 outputs are modelled crop-water responses for supported herbaceous crops and matched inputs. WaPOR v3 is remote-sensing water-productivity context at its published spatial/temporal support. Neither is a plot-level observation simply because its geometry overlaps a field.

### 4.4 Farmer choice, action and outcome
- farmer decision and consent are attributable to a scoped actor and time, and retained without a trust penalty for decline/defer;
- local draft, prepared, sent, OutcomeUnknown, acknowledged and reconciled states remain distinct from provider service lifecycle states;
- action execution requires a separate authorized record; projection never executes it;
- provider-reported completion, independent inspection and farmer acceptance/dispute remain separate;
- outcome records link to, but do not automatically become causal evidence for, advice or action;
- corrections/retractions append lineage and preserve the historical source claim.

## 5. Evidence rights and privacy

Access permission, reference permission, redistribution permission, evaluation permission and training permission are distinct. Preserve source-specific rights and current permission state with each source artifact or dataset version.

A source listed in CropFoundry's registry as training-disabled or quarantined remains so after projection. A readable local file or publicly downloadable product is not an authorization to train, redistribute or infer new personal facts.

The first conformance suite uses synthetic data only. Do not include real farm coordinates, farmer names, phone numbers, commercial terms, patient/worker details or precise location histories. Production integrations require purpose limitation, data minimization, consent/legal basis, retention/deletion, tenant isolation, export controls and threat review.

## 6. External references and claim ceilings

- [FAO AquaCrop 7.3](https://www.fao.org/land-water/news/detail/fao-releases-the-new-aquacrop-version-7.3/en) was announced on 31 July 2026 with the first Python source implementation and refinements including soil hydraulic-property estimation and simulation processes. It models yield response to water for supported herbaceous crops; it is not a universal farm or soil-flow model.
- [FAO WaPOR v3 API](https://www.fao.org/in-action/remote-sensing-for-water-productivity/wapor-data-access/en) supplies remote-sensing-derived water-productivity data. Preserve level, product, pixel/resolution, time aggregation and coverage. It is contextual evidence, not plot-scale ground truth.
- [Symtropy soil-water accounting PR #2848](https://github.com/Luminous-Dynamics/symtropy/pull/2848) accounts for supplied interval water fluxes and storage. It does not independently predict ET, runoff, drainage, Richards flow, crop growth or agronomic efficacy.
- [Mycelix NEON grouped transfer issue #5534](https://github.com/Luminous-Dynamics/mycelix/issues/5534) concerns preferential-flow proxy/measurement transportability and site-held-out evaluation. A proxy velocity is not automatically tracer truth or a crop/amendment outcome.

Comparisons are task-specific. An accounting identity cannot be described as outperforming AquaCrop. A regional WaPOR product cannot be scored as if it were a plot sensor. If model, target, unit, time, spatial support, crop or data population differ, report not-comparable.

## 7. Conformance lanes

The synthetic vector collection in CONFORMANCE-VECTORS.json exercises:

- measured soil observation with explicit depth, method, unit, source and time;
- forecast kept distinct from weather observation;
- modelled recommendation with premise lineage and zero execution authority;
- farmer-declined advice without trust penalty or fabricated outcome;
- action with unresolved outcome retained as unknown, not zero/success/failure;
- conflicting source observations retained together without a silent winner;
- training-disabled/quarantined data rights preserved and never promoted;
- cross-tenant plot reference rejection;
- missing/incompatible measurement unit made non-evaluable;
- modelled water-balance result never promoted to measured soil moisture;
- correction/retraction appended without overwriting the original source statement.

The vector schema validates the fixture document structure only. Neither the profile nor its vectors are a normative OMSI wire schema or an executed adapter.

## 8. Required independent checks

A future adapter implementation must:
- account for every observed source field as preserved, transformed with a registered transform, explicitly dropped with reason, or unmapped; no silent loss;
- reject duplicate/unknown IDs and cross-scope references;
- bind input and output artifacts to exact byte digests, profile/evaluator version and source permissions;
- preserve chronology and source-vs-effective time semantics without inventing timezone information;
- retain unknown/stale/conflicting/quarantined states;
- independently rebuild the projection and mapping coverage without calling the candidate adapter;
- prove that no action, notification, procurement, booking, payment, training or actuation is performed by the reader.

A successful synthetic vector check qualifies only the stated projection semantics. It does not establish agronomic accuracy, farmer usability, field efficacy, causal benefit, data permission, legal compliance or production readiness.

## 9. Acceptance criteria for profile 0.1

- All positive fixtures preserve the specified source/domain distinctions and expected claim ceiling.
- All negative controls reject or remain explicitly unavailable/inconclusive for the intended reason.
- No missing unit, unknown denominator, quarantined source or unobserved outcome becomes a favorable numeric result.
- Farmer decline/defer is preserved without penalty or implied adverse outcome.
- Every modelled or forecast result carries model/source/time identity and remains non-measured.
- Cross-tenant or unauthorized scope never projects.
- A later implementation is independently verified against the full vector set before being considered for E1; no E2/E3/field claim is implied here.

## 10. Decision

Admit this as an OMSI agriculture/farmer profile incubation document. Keep all agronomic models, advice logic, farmer workflow, offline recovery and source-specific permissions with their current owner projects. The next implementation step is a read-only profile adapter and independent conformance verifier, not another crop, soil or farmer application.
