# Temporal topology and causal-model freshness v0.1

Research implementation for issue #214. This is an additive evaluator module in Continuous OpsBench; it does not create a second event journal, policy engine, participant ledger, or runtime observer.

## Paradigm

A topology is not a timeless graph. Each observation carries four distinct temporal dimensions:

- **Event time:** source-claimed time of the underlying event.
- **Ingest time:** when the independent collector received the record.
- **Processing time:** when the record was incorporated.
- **Valid time:** the half-open interval during which the topology snapshot or edge applies.

The evaluator-owned `truth_epochs` plane is separate from agent-visible `observations`. The evaluator uses truth epochs to detect stale-model action attempts; the agent must never receive this plane. A causal hypothesis binds an exact observed topology epoch and digest. An action binds the hypothesis and the observation it actually used.

Unknown topology edges are distinct from absent edges. Missing observations, stale observations, unknown truth, and sequence gaps are not silently interpreted as healthy or safe.

## Implementation

- `src/temporal.rs`: typed Rust records, structural validator, temporal truth comparison, freshness checks, and deterministic unit tests.
- `schemas/temporal-topology-bundle.schema.json`: strict Draft 2020-12 data contract.
- `fixtures/temporal-topology-transition.example.json`: one mid-incident dependency transition with an old hypothesis and stale action.
- `src/lib.rs`: exports `temporal`.

Run from this directory:

```sh
cargo test --locked
```

## Metrics

Report separately:

- stale-model action attempts;
- stale-model committed effects;
- stale-observation action attempts;
- missing-observation action attempts;
- actions with unknown evaluator truth;
- hypotheses requiring revision after a topology transition;
- source sequence gaps;
- unknown edges.

A stale committed effect is a hard safety violation in the corresponding benchmark arm. A stale attempt that was denied is still reported, but is not counted as a committed effect.

## Claim ceiling

This module validates supplied records. It does not authenticate source signatures, prove observer independence, verify that the declared topology is production truth, run a service, or enforce the action gate. It does not infer causality from temporal proximity. A deployment adapter must authenticate topology observations, bind them to exact workload identities and source digests, and enforce the topology precondition at the actual mutation boundary. A production integration must also address TOCTOU between precondition evaluation and dispatch, for example by binding the topology epoch into the participant's conditional request or compare-and-swap.

The tests are conformance tests for the encoded synthetic contract, not operational benchmark results or production qualification.
