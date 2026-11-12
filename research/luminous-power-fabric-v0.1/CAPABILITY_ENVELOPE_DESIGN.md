# Evidence-gated electrical capability envelope v0.1

**Snapshot:** 2026-11-12  
**Status:** read-only software design and screening implementation; no device qualification or field capability is claimed  
**Parent:** [#416 — evidence-gated electrical port capability evaluator](https://github.com/Luminous-Dynamics/luminous-platform/issues/416)  
**Field-system parent:** [#413 — portable industrial field-power router](https://github.com/Luminous-Dynamics/luminous-platform/issues/413)

## 1. Why a static rating is not a commitment

A component can have a 1 kW nameplate rating but be unable to credibly deliver 1 kW now because its qualified configuration changed, the thermal margin collapsed, a required sensor is invalid, the evidence expired, telemetry is stale or an independent protection subsystem is not ready.

The v0.1 evaluator answers a deliberately narrow question:

> Given a caller-supplied evidence snapshot and declared engineering bounds, what continuous forward and reverse active-power capability may this exact configuration expose to a higher-level planner?

It does not decide whether the physical device is safe. It does not command the device. It does not replace a breaker, isolator, protection relay, local controller or site engineer.

## 2. Typed input and deterministic gate sequence

The Rust implementation is in:

- `tools/measurement-audit/src/capability_envelope.rs`

The input and example contracts are:

- `schemas/capability-envelope-v0.1.schema.json`
- `schemas/capability-envelope.synthetic.json`

Evaluation sequence:

1. Validate direction-specific continuous ratings, uncertainty bounds, derating factor, time values and non-empty provenance references.
2. Validate both configuration SHA-256 strings and require the observed configuration to match the qualified configuration.
3. Require qualification validity at the evaluation time.
4. Require current evidence, calibration evidence and sensor evidence.
5. Require independent protection state to be Ready.
6. Require telemetry to exist, not be future-dated and remain within its declared age limit.
7. Validate the thermal measurement boundary and derating interval.
8. Calculate the declared thermal factor.
9. Apply the external installation/source/environment factor.
10. Deduct conservative absolute measurement and model uncertainty bounds.
11. Return forward/reverse available power and all explicit blockers.

A single failed hard gate withdraws both directional capability values. The evaluator never averages a failed safety/evidence gate against a performance improvement.

## 3. v0.1 equations and assumptions

The first thermal screen is piecewise linear:

```text
f_T = 1                                              when T <= T_derate
f_T = (T_inhibit - T) / (T_inhibit - T_derate)      when T_derate < T < T_inhibit
f_T = 0 and BLOCK                                    when T >= T_inhibit
```

For each direction d:

```text
P_available,d = max(0, P_rated,d * f_T * f_external
                       - U_measurement - U_model)
```

Here, U_measurement and U_model are supplied conservative absolute uncertainty bounds in watts. They are not standard deviations, confidence intervals, or automatically calibrated quantities. Their provenance and validity must be independently justified by the caller.

The thermal law is a transparent screening approximation. It is not a semiconductor junction-temperature model and does not account for thermal lag, transient safe operating area, component-specific non-linear derating, heat-soak history or cooling-system dynamics. The measured temperature boundary must be declared (case, heatsink, winding, ambient, etc.) and cannot be silently interpreted as junction temperature.

The external factor represents caller-supplied constraints such as source, cabling, installation or ambient derating. v0.1 does not calculate this factor from cable geometry, connector ratings, source impedance, altitude or enclosure airflow. A production integrator must bind it to reviewed engineering inputs and evidence.

## 4. Hard blockers and claim ceiling

The implementation returns explicit blockers for:
- Malformed or mismatched configuration digest.
- Expired qualification.
- Non-current evidence, calibration or sensor status.
- Protection not ready (tripped, faulted, bypassed or unknown).
- Missing, future-dated or stale telemetry.
- Invalid thermal interval or reached thermal inhibit point.

It rejects non-finite/negative ratings and uncertainty, invalid external factors, invalid timing values, missing provenance references and overflow in the combined uncertainty bound.

Claim ceiling:

`EVIDENCE_GATED_CAPABILITY_SCREEN_ONLY_NOT_CONTROL_OR_PHYSICAL_QUALIFICATION`

A valid schema or a successful function call does not prove that a sensor is accurate, the model is correct, the configuration is authentic, the protection hardware works or the power can physically be delivered. This library does not authenticate caller-provided digests or evidence states; an upstream evidence verifier must establish those facts.

## 5. Benchmark programme

The benchmark must prioritize false availability over maximum claimed capacity.

### Hard negative-control suite

Mutate one field at a time and in combinations:
- Hardware/firmware/config digest mismatch.
- Missing, expired or invalid qualification.
- Invalid sensor calibration.
- Sensor disconnected, saturated or polarity-reversed.
- Protection tripped, bypassed, faulted or unknown.
- Missing, stale or future timestamp.
- Thermal inhibit reached or invalid temperature boundary.
- Uncertainty bounds missing, invalid or inflated.
- Model / external derating evidence withdrawn.
- Power-flow direction mismatch.

**Target:** zero non-zero capability outputs for every frozen hard-negative control. This is a software-invariant target, not a physical safety probability or certification result.

### Ground-truth / conservative-availability tournament

Compare four arms using identical operating episodes:
1. Nameplate-only availability (intentionally unqualified baseline).
2. Thermal-only derating.
3. Thermal + external derating.
4. Full configuration/evidence/telemetry/uncertainty gate.

Report:
- False-available event count (non-compensatory hard failure).
- False-unavailable capacity and its root cause.
- Available-versus-independently-measured deliverable power.
- Power withheld by each explicit blocker.
- Direction-specific results.
- Time from evidence invalidation to capability withdrawal.
- Requalification latency after a changed configuration or repaired component.
- Decision latency, CPU/memory cost and deterministic replay equivalence.

Proposed software targets for an explicitly named reference host:
- One capability evaluation below 1 ms at p99.
- Batch evaluation of 10,000 independent port snapshots below 100 ms at p99.
- Deterministic identical outputs for identical canonical inputs and evaluator revision.
- Zero false-available outputs in all frozen hard-negative controls.
- In truth-labelled eligible episodes, investigate a false-unavailable capacity target below 5%; do not optimize it until zero false-available behavior and evidence integrity are preserved.

These latency and false-unavailable figures are research targets, not achieved results or safety limits. Reference hardware, compiler, optimization profile, input corpus, clock source and timing method must be pinned before reporting results.

### Dataset / cluster discipline

- Freeze operating episodes before tuning derating policies.
- Separate by site, asset and time period; avoid random splitting of adjacent samples from one physical event.
- Keep high/low temperature, both flow directions, load step, cooling mode, source condition and repair/configuration change as explicit strata.
- Use independent measurement boundaries and preserve raw waveform and instrument lineage.
- Publish every failed, unknown, aborted and excluded episode with reason.
- Do not claim calibrated uncertainty until the supplied bounds have been validated against independent data.

## 6. How this becomes more powerful in v0.2+

v0.1 is a continuous active-power snapshot. It is intentionally not a multi-physics dispatch optimizer. The next envelope should be a tensor over:
- Time and direction.
- Voltage/current and apparent/reactive power where AC applies.
- Ramp rate, response notice and transient overload duration.
- Sustained delivery interval and recovery / rebound.
- Thermal state and cooling availability.
- Battery state-of-charge, state-of-health and cycle/temperature limits.
- Source availability and source impedance.
- Branch, busbar and connector current limits.
- Power quality, fault-current and grounding constraints.
- Common-cause dependency and correlated failure domains.

A system-level capability claim must not simply add individual ratings. The aggregate limit must account for shared upstream buses, common cooling, common controllers, common source capacity, protection zones, and any single failure that removes multiple modules. The future system evaluator should calculate both the capability envelope and the minimal cut sets that invalidate it.

Only after the deterministic capability contract is sound should Symtropy-derived predictive thermal or degradation estimates contribute additional uncertainty. The measurement envelope must retain an independently measured path.

## 7. Integration across Luminous systems

| System | Input / responsibility | Non-authority boundary |
|---|---|---|
| Symthaea | Use [SE-OPT-BENCH-001 #7803](https://github.com/Luminous-Dynamics/symthaea/issues/7803) for decision-impact-driven experiment selection; use existing energy architecture work as the design-search owner | Cannot change qualification status or waive a hard blocker |
| Symtropy | Compose [POWER-SYSTEM-DIGITAL-TWIN-001 #3400](https://github.com/Luminous-Dynamics/symtropy/issues/3400), [POWER-SYSTEM-RESILIENCE-001 #3418](https://github.com/Luminous-Dynamics/symtropy/issues/3418), topology #3409 and matched-service FFER #3417 for physical energy, islanding, restoration and duty-cycle replay | Model output does not become measurement evidence without correlation |
| Sol-Atlas | Compose [ENERGY-GEN-PORTFOLIO-001 #293](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/293), [ENERGY-RESOURCE-SITE-001 #662](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/662) and [GMD-RESILIENCE-001 #692](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/692) for source/site context, spatial exposure and infrastructure dependencies; bind equipment geometry/revision where available | CAD / spatial geometry does not prove thermal/electrical limits |
| Mycelix | Compose [instrument capability/calibration #6034](https://github.com/Luminous-Dynamics/mycelix/issues/6034), [electrical channel taxonomy #6029](https://github.com/Luminous-Dynamics/mycelix/issues/6029), [condition applicability #6065](https://github.com/Luminous-Dynamics/mycelix/issues/6065), [stream/timebase #6054](https://github.com/Luminous-Dynamics/mycelix/issues/6054), [correlated uncertainty #6048](https://github.com/Luminous-Dynamics/mycelix/issues/6048) and [dependency invalidation #6042](https://github.com/Luminous-Dynamics/mycelix/issues/6042). #6029 is a building/habitat channel extension; industrial power measurands need an explicit product-specific profile | Signatures and ledger inclusion do not prove physical truth |
| Luminous Platform | Versioned contracts, capability evaluation, lifecycle/requalification graph, audit outputs and downstream planning interface | Does not command the power path or bypass local protection |
| Local embedded control | Enforce bounded operating modes and deterministic regulation | Not replaced by this offline evaluator |
| Independent protection | Detect/trip/interrupt/isolate faults using qualified hardware | Remains the physical safety authority |

The evidence flow is:

```text
Physical observation / supplier evidence
       -> instrument and configuration binding
       -> independent evidence verification
       -> capability-envelope evaluation
       -> bounded planner visibility
       -> local controller accepts only within its own limits
       -> independent hardware protection remains authoritative
       -> measured outcome / fault / repair
       -> requalification graph
```

## 8. Immediate next experiments

1. Add an explicit capability schema verifier and canonical input/output digesting to the audit tool.
2. Generate a 100-case synthetic mutation corpus for every gate and pairwise gate interaction.
3. Bind the thermal factor to Symtropy replay traces, while preserving a model-only evidence label.
4. Add Sol-Atlas configuration references for package, heatsink, fan, cable and connector revisions.
5. Add Mycelix evidence references for sensor calibration and qualification expiry; require independent reviewer identity where appropriate.
6. Compare capability-withdrawal time and false-unavailable rate across nameplate, thermal-only and evidence-gated policies.
7. Extend from one port to a multi-port power fabric with common-cause cut-set and recovery analysis.

## 9. Validation status

Authored Rust unit tests are present in the module. The current execution environment did not provide cargo, rustc, rustfmt or nix during this task; compilation, test execution, formatting and Clippy have not been performed. JSON Schema and example artifacts must be run through a full Draft 2020-12 validator before claiming schema validation. No hardware, field, protection, safety or certification tests have been run.
