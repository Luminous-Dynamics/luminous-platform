# Continuous OpsBench v0.1

Research-only deterministic evaluator for issue #188. This is the continuous-operation companion to episodic OpsBench, not a replacement for it.

## What is evaluated

The evaluator consumes separate planes:
- evaluator-owned fault truth and active/revert intervals;
- agent-produced incident reports and proposed/executed intervention references;
- participant-owned attempt/effect receipts;
- independent service observations bound to a frozen offered-demand workload profile;
- a no-action twin using the same scenario schedule and workload identity.

It derives per-fault correlation, exact service/artifact localization, repair before evaluator cleanup, sustained workload-conditioned recovery, false repair claims, intervention safety, duplicate committed effects, unauthorized committed effects, and demand-weighted degraded request-time.

Every injected fault remains in the denominator, including faults that disappear before an agent invocation. No-action observations are compared only when bound to the same scenario and workload profile. Metrics remain separate; there is no compensatory composite score.

## Run

```sh
cargo test --locked
```

No external crates or live infrastructure are used.

## Claim ceiling

This is a typed offline evaluator over supplied records. It does not generate Kubernetes faults, run an agent, authenticate publishers, verify cryptographic signatures, prove the independence of declared observer sources, or establish production representativeness. A committed participant receipt is trusted as participant-owned input. A production adapter must bind exact episode/run/arm and evidence digests, use a source-authenticated participant ledger and observer, and integrate through Platform Saga and the existing effect broker. Nixward remains the sole host transaction and recovery authority.

The synthetic fixture is not a benchmark result and cannot support SOTA or production-safety claims.


## Temporal topology and causal-model freshness v0.1

The additive `src/temporal.rs` evaluator separates evaluator-owned topology truth from agent-visible topology observations. It binds causal hypotheses to exact topology epochs/digests and reports stale-model action attempts, stale committed effects, stale or missing observations, unknown truth, sequence gaps, and unknown edges. Event time, ingest time, processing time, and valid time remain separate. See `TEMPORAL_TOPOLOGY_V0_1.md`, `schemas/temporal-topology-bundle.schema.json`, and the transition fixture.

This is a structural synthetic evaluator. It does not authenticate topology sources or enforce action preconditions at the participant boundary. A production adapter must bind the epoch into a conditional participant operation to address time-of-check/time-of-use races.


## Participant-side conditional mutation v0.1

The additive \`src/conditional.rs\` reference model serializes topology publication and mutation commit through one participant-owned mutex. A mutation is accepted only when its expected topology epoch and digest still match at the commit boundary, and the authorization decision is bound to the exact participant, scope, tenant, attempt, logical action, operation, target, request digest, and expected topology epoch+digest.

The effect journal is idempotent by tenant plus logical action key. An exact retry returns the original receipt even after topology advances; reusing the key for a different request is rejected. A commit receipt records the topology epoch/digest that was current at the linearization point.

The tests include stale-epoch and same-epoch/different-digest rejection, denial of authorization retargeting to a newer topology epoch, successful fresh authorization at that epoch, exact retry, idempotency conflict, authorization mismatch/denial/expiry, monotonic topology publication, and a concurrent topology-update/commit race.

**Claim ceiling:** this is an in-memory concurrency reference model. The mutex is not a durable database transaction, a distributed CAS, or a production authorization verifier. A production participant must put the topology compare, effect write, and idempotency journal in the same durable serializable transaction or equivalent linearizable conditional-write primitive. Authorization signatures and trust roots must be authenticated before the normalized authorization receipt reaches this module. No production safety claim follows from the synthetic tests.
