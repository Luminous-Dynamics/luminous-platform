# Common-cause-aware multi-path capability v0.1

**Snapshot:** 2026-11-12  
**Status:** deterministic software screen and synthetic test suite; not a physical network model or reliability qualification  
**Parent research:** [#417 — common-cause-aware multi-port capability and recovery](https://github.com/Luminous-Dynamics/luminous-platform/issues/417)  
**Single-path prerequisite:** [#416 — evidence-gated electrical port capability](https://github.com/Luminous-Dynamics/luminous-platform/issues/416)  
**Field architecture:** [#413 — portable industrial field-power router](https://github.com/Luminous-Dynamics/luminous-platform/issues/413)

## 1. The failure of nameplate addition

If two 100 W paths share one 48 V source, busbar, controller, cooling loop or branch protection device, they do not provide 200 W of independent N-1 capability. A single failure in that shared dependency may remove both paths.

Likewise, two converters with separate power stages can still share:
- one upstream battery inverter or generator;
- one common DC link or AC feeder;
- a single enclosure or cooling loop;
- common gate-drive/control power;
- one protective device or maintenance isolator;
- common firmware/configuration or common sensor references;
- the same cable tray, connector panel or environmental exposure.

A correct system-capability calculation must model these declared dependencies explicitly.

## 2. v0.1 implementation

The Rust module is:

- `tools/measurement-audit/src/multi_port_capability.rs`

Public API:

- `evaluate_multi_path_capability`

Input concepts:
- `service_boundary_id`: one exact useful-service boundary shared by every route; path powers cannot be added if they terminate at different buses, load groups or measurands.
- `CapabilityPath`: one already-prequalified source-to-service route, its continuous bottleneck power, availability state, exact configuration SHA-256, capability-evidence reference, component/dependency IDs and shared failure-domain IDs.
- `DeclaredFailureScenario`: one explicit multi-component and/or multi-domain contingency.
- `MultiPathCapabilityInput`: service boundary, path set, additional contingencies and optional useful-service threshold.

Only paths marked `QualifiedAvailable` contribute baseline capacity. The caller must supply a route bottleneck, not add series-component ratings. A non-available status (evidence blocked, protection not ready, faulted or unknown) contributes zero even if a stale positive power number was passed.

The evaluator automatically creates a deterministic single-failure scenario for every unique component ID and every unique shared failure-domain ID. If a path contains that failed ID, the entire path is removed. A component or domain shared by several paths therefore removes all of them together.

Additional declared scenarios permit N-k and selected common-cause combinations. All scenario references must resolve to IDs in the input graph. Duplicate paths, duplicate IDs, unknown scenario dependencies, missing components, negative/non-finite power and arithmetic overflow are rejected.

The result contains:
- the shared service-boundary identity;
- each path's exact configuration digest and upstream capability-evidence reference;
- baseline sum of currently qualified parallel route bottlenecks;
- capability remaining after each generated and declared contingency;
- the minimum surviving (robust) power;
- the limiting contingency identifier;
- optional comparison against a required useful-service power threshold;
- all per-scenario path removals.

The result is deterministic under path-array permutation; scenario ordering and tie-breaking are stable.

## 3. Mathematical interpretation

For path (p), let (P_p) be its already-qualified continuous power bottleneck. Let (A_p) indicate whether the upstream evidence/protection gate marks the path currently available. For contingency (c), a path survives only if it is available and none of its declared components or failure domains are failed by (c).

```text
P_base = sum(P_p for every available path p)

P(c) = sum(P_p for every available path p that survives contingency c)

P_robust = min(P_base, P(c) for all evaluated contingencies)
```

The evaluator does not assign probabilities to contingencies. This is deterministic contingency analysis. If the system topology contains an undeclared common dependency, the model will not discover it. No reliability probability or safety integrity claim is inferred.

## 4. Why this is only a first layer

v0.1 is a parallel-route capacity screen. It does not yet:
- calculate serial path bottlenecks from node-level component limits;
- solve AC power flow, voltage drop, impedance, phase, reactive power or transient stability;
- calculate cable/busbar ampacity, thermal coupling, source impedance, fault current, I²t, selectivity or protection clearing;
- model energy storage SOC, transient reserve, ramp, response duration, recharge or recovery/rebound;
- infer common-cause failure domains from physical geometry or software/configuration similarity;
- generate all minimal cut sets for a multi-state fault tree;
- compute component failure probabilities, reliability, availability or confidence intervals;
- decide whether an operator is permitted to switch, isolate, reclose or maintain hardware.

Those are distinct owners and future integration tasks. The caller must provide a complete, reviewed declared dependency list and route capacities already bounded by the upstream port capability and engineering analysis. All route powers must be expressed at the same declared useful-service boundary. Each route must bind the exact upstream configuration digest and capability-evidence reference; the evaluator validates digest shape and preserves identity but does not authenticate the evidence.

## 5. Required common-cause identifiers

When composing real field equipment, component IDs and failure-domain IDs should at least consider:

| Dependency class | Example identifier meaning | Typical effect |
|---|---|---|
| Source | one battery inverter, generator or utility feeder | removes every route powered by that source |
| Electrical trunk | one DC link, busbar section or AC feeder | removes all downstream routes |
| Protection | common upstream fuse, breaker, contactor or isolation device | removes all protected downstream branches |
| Cooling | shared fan, pump, coolant loop, radiator or HVAC | removes routes depending on that thermal resource |
| Control power | common auxiliary supply or gate-drive bias rail | may disable several control paths |
| Firmware/configuration | common software image or configuration revision | common-mode defect may affect multiple modules |
| Measurement | shared reference/transducer/clock chain | can invalidate multiple capability claims even while power hardware remains available |
| Enclosure / environment | common ingress, fire, vibration or thermal exposure zone | can remove multiple physically adjacent modules |
| Maintenance boundary | common connector panel, isolation point or service operation | a planned maintenance action can remove several paths |

Do not infer independence merely because components have different serial numbers or are on separate PCBs. Failure-domain identity requires design and operational evidence. If uncertain, the dependency remains shared or unresolved; do not credit redundant capacity.

## 6. Benchmark programme

Compare the following methods on frozen synthetic and later independently measured topologies:

1. **Nameplate sum:** sums all route power without dependency checks. Deliberately naive baseline.
2. **Independent-module N-1:** removes one module at a time while incorrectly assuming all other modules remain independent.
3. **Component-ID N-1:** removes every route that uses the failed component ID.
4. **Shared-domain N-1:** removes every route sharing a declared common-cause domain.
5. **N-k / compound contingencies:** includes caller-declared combinations such as source + cooling loss or bus + protection unavailability.
6. **Recovery-aware system contract:** future extension incorporating fault detection, verified isolation, repair, requalification, restart, SOC recovery and rebound.

Report:
- baseline versus robust available capacity;
- capacity lost per dependency and contingency;
- false-available system-capability events (hard failure);
- false-unavailable capacity and cause;
- false redundancy credit from the nameplate and independent-module baselines;
- all scenarios, including zero-capacity outcomes;
- path and dependency graph lineage;
- runtime / memory versus path and contingency count;
- deterministic replay equivalence under input permutation;
- exact relation between required useful service and robust capacity.

Proposed software targets (not achieved):
- Zero non-zero robust capacity when every path is removed by a declared single failure or common-cause cut set.
- Zero false-available outputs across a frozen deterministic contingency corpus.
- 100% path, configuration, capability-reference, component and domain lineage in every result.
- Deterministic output under input permutation.
- Batch runtime target under 100 ms at p99 for 10,000 paths and 100,000 declared/generated scenarios on a pinned reference host; test and publish the actual scaling curve instead of treating the target as a pass.
- No probability-weighted reliability score without a separately qualified probability model and empirical evidence.

## 7. Test suite

The Rust module currently authors 24 unit tests, including:
- independent parallel paths and N-1;
- shared source failure;
- shared cooling domain;
- independent failure domains;
- path blocked by unavailable protection/evidence;
- shared controller failure;
- declared compound cooling failure;
- useful-service threshold compared against robust—not baseline—power;
- unavailable path excluded from baseline;
- empty/duplicate paths and duplicate component references;
- missing component IDs;
- invalid/negative/non-finite power and summation overflow;
- unknown component/domain references;
- empty/duplicate scenarios;
- invalid required-service threshold;
- claim ceiling;
- deterministic scenario order under path permutation.

These tests are authored, not executed. They are synthetic arithmetic controls, not physical or reliability evidence.

## 8. Cross-project ownership

| Owner | Contribution |
|---|---|
| Symtropy [#3400](https://github.com/Luminous-Dynamics/symtropy/issues/3400) / [#3418](https://github.com/Luminous-Dynamics/symtropy/issues/3418) | Generator/storage/load dynamics, islanding, black-start, fault/restoration replay and matched useful-service accounting |
| Symthaea [#7803](https://github.com/Luminous-Dynamics/symthaea/issues/7803) | Choose the next physical or simulation experiment by decision impact under a fixed budget |
| Sol-Atlas [#662](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/662) / [#692](https://github.com/Luminous-Dynamics/sol-atlas-leptos/issues/692) | Site/source context, spatial exposure, physical adjacency and infrastructure dependencies |
| Mycelix [#6034](https://github.com/Luminous-Dynamics/mycelix/issues/6034) / [#6048](https://github.com/Luminous-Dynamics/mycelix/issues/6048) / [#6042](https://github.com/Luminous-Dynamics/mycelix/issues/6042) | Instrument, calibration, timebase, correlated uncertainty, exact evidence dependency and invalidation |
| Luminous Platform | Deterministic composition, graph schema, capability and lifecycle receipts |
| Local controller / hardware protection | Actual regulation, trip, interruption and isolation; never delegated to these read-only evaluators |

## 9. Claim ceiling and validation state

`DECLARED_CONTINGENCY_CAPABILITY_SCREEN_ONLY_NOT_RELIABILITY_OR_PHYSICAL_QUALIFICATION`

The module does not switch or control hardware and does not certify the completeness of the dependency graph. Compilation, tests, formatting and Clippy require a Rust-capable runner and remain unverified in this work increment. No energized hardware, fault injection, field, reliability or certification test is claimed.
