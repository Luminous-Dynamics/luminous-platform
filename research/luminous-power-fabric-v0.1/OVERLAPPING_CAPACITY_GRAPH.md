# Power Fabric overlapping shared-capacity graph — v0.1

**Tracking:** [#451 — topology-aware shared-capacity graph and common-cause max-flow qualification](https://github.com/Luminous-Dynamics/luminous-platform/issues/451)  
**Existing evaluator:** `tools/measurement-audit/src/multi_port_capability.rs` (v0.1 parallel-path screen)

**Status:** two synthetic graphs with manually reconciled max-flow cases. No Rust evaluator, network physics, reliability probability, HIL or physical qualification is implemented.

## 1. What the fixture tests

The existing multi-path evaluator safely rejects partial-overlap path groups. This corpus makes that limitation concrete and provides a future acceptance oracle.

Graph one has three source-to-sink routes with:
- A/B sharing a cooling dependency;
- B/C sharing a control-power dependency;
- all three sharing one upstream feeder;
- all three sharing one downstream service bus.

Every arc capacity is expressed as conservative net active power at the same customer POC. These are synthetic service-side capacities, not physical component ratings.

| Arc | Directed route | Capacity at service POC |
|---|---|---:|
| A | source A → junction | 80 W |
| B | source B → junction | 70 W |
| C | source C → junction | 60 W |
| Shared feeder | junction → feeder output | 170 W |
| Service bus | feeder output → customer POC | 160 W |

The naive path sum is 210 W. The baseline maximum flow is 160 W. This is not a measured capability; it is a small graph oracle.

## 2. Expected contingency outcomes

| Scenario | Surviving source-side capacity | Expected maximum service flow |
|---|---:|---:|
| Baseline | 210 W | 160 W |
| A/B shared cooling failure | C only: 60 W | 60 W |
| B/C shared control failure | A only: 80 W | 80 W |
| Source A component failure | B+C: 130 W | 130 W |
| Common feeder failure | 0 W | 0 W |
| Service bus failure | 0 W | 0 W |

The second graph makes arc C's evidence state `unverified`. That arc is non-contributing even though its declared capacity is positive; the expected maximum flow falls to 150 W. Unknown evidence is not a usable capacity.

## 3. Why max-flow/min-cut is the next mathematical step

The graph represents each shared resource once. Multiple paths that traverse the same feeder consume that same edge capacity, so the algorithm cannot count the same physical shared capacity multiple times. Partially overlapping dependency sets are represented by paths that traverse overlapping graph edges/domains rather than by disjoint/identical path-group annotations.

For each scenario, the expected result includes a minimum-cut witness. A future evaluator must return both the scalar capacity and the limiting cut/contingency evidence.

## 4. Hard scope boundary

This is not an AC or DC power-flow solver. It does not infer voltage drop, impedance, current, phase, frequency, reactive power, conversion behavior, protection coordination or thermal dynamics.

All arcs are normalized to a common owner-qualified net active-power basis at the exact customer POC. If a source/branch/converter limit cannot be defensibly expressed at that boundary, reject the graph. Do not substitute an unverified efficiency guess.

A graph flow is a deterministic capacity upper-bound under declared inputs. It is not necessarily physically feasible where electrical laws, voltage/current limits, dynamics or protection intervene. Symtropy and specialist electrical owners remain responsible for those models and evidence.

## 5. Files and validation

- `schemas/overlapping-capacity-graph-corpus-v0.1.schema.json`: strict Draft 2020-12 structural contract.
- `schemas/overlapping-capacity-graph-corpus.synthetic.json`: two graph fixtures and seven expected scenarios.
- Existing `multi_port_capability.rs` is not modified.

Authoring-time arithmetic recomputation on this small topology must verify the expected source sums and serial bottlenecks. A future Rust implementation must additionally compare against an independent max-flow/min-cut oracle and exhaustively enumerate small graphs. No SOTA, network, product, safety or reliability claim follows from this fixture.
