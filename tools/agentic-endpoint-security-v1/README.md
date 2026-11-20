This crate intentionally has no third-party dependencies. Both evaluators accept normalized model facts and are **not** cryptographic verifiers, operating-system observers, policy enforcement components, production authorization services, or security proofs. Synthetic example output must not be counted as benchmark execution evidence.

The standard-library Python checkers are test tooling only: one freezes the 20 fixed-corpus IDs/outcomes and lab constraints; the other freezes adaptive attack families, query budgets, comparison arms, sample-size rules, and anti-gaming constraints. Mutation tests ensure key adaptive protocol requirements cannot silently be weakened. The PR workflow uses pinned action commits, Rust 1.96.0, offline Cargo test/build steps, and read-only repository permissions.

The Rust suite contains **28 authored tests** across the evidence-dimension evaluator and [ordered trace oracle](../../docs/ENDPOINT_TRACE_ORACLE_V1.md), including binding-field mutations, trace-order/replay/gap/no-effect cases, and an exhaustive 29,160-state contract matrix. Authored tests have not been compiled or run locally; use the exact-head workflow result before marking them passed. The models compare exact tuples and trace order but do not authenticate sources, consume nonces durably, observe real OS effects, or prevent actions. Runtime prevention still requires a T1+ lab adapter and an independent effect observer.

## Ordered trace oracle and episode scoring

- [Ordered trace oracle v1](../../docs/ENDPOINT_TRACE_ORACLE_V1.md) defines the normalized grant/deny/attempt/revocation/effect/checkpoint model and its claim limits.
- [Episode scoring v1](../../docs/ENDPOINT_EPISODE_SCORING_V1.md) documents the JSONL result contract and interpretation of adaptive violation rate, assurance failure rate, false refusal, evidence gaps, and exact confidence bounds.
- `score_episodes.py` validates submitted episodes and emits stratified JSON metrics. Only `valid` episodes contribute to efficacy-rate denominators; `incomplete` or `invalid` episodes after a verified start count as assurance failures, not clean outcomes. Any confirmed forbidden effect in an incomplete/invalid episode still fails the critical hard gate. The scorer requires independent known outcomes to be explicitly marked, prevents cumulative forbidden effects from disappearing at later budgets, and does not qualify a zero-event critical claim unless the per-stratum denominator reaches 2,995 with no uncertainty.
- A prior scorer prototype passed 20 standard-library unit tests. The scorer was subsequently refactored to separate independent oracle outcomes from system dispositions and require canonical digests for the runtime/configuration/task/scenario identity; its current committed suite has 30 authored tests, including ten new regressions, which have not been run locally. The 10 oracle-contract mutation tests are also unrun locally. Exact-head CI is required; no real endpoint or adaptive attack result is implied.

- [Independent oracle fidelity v1](../../docs/ENDPOINT_ORACLE_FIDELITY_V1.md) and `oracle-coverage.json` freeze 10 side-effect channels, five observer roles, and 10 observer-fault controls. The manifest checker and its mutation tests are wired into CI; they validate the contract, not actual OS observer execution.


## Trajectory-wide effective privilege model

`src/trajectory.rs` adds an independent typed model for task-required, granted, available, exercised and independently observed authority across a trajectory. It evaluates delegated grant attenuation, parent-to-child revocation, event-time grant validity, excess exposure, unauthorized admission/effects, observer gaps and false refusals. The model is normalized-fact logic only; it does not discover tool aliases, authenticate grant tokens, observe real effects or enforce runtime ACLs. Its tests run as part of the existing dependency-free Cargo test job.


## Canonical authority path closure (CAPC)

The existing read-only benchmark workflow also runs the standard-library CAPC checker and mutation tests. CAPC qualifies owner-issued resource/effect identity across independently inventoried MCP/REST/SDK/direct paths. It is not a policy engine, resource registry, route discovery service, or effect observer. See [CAPC v0.1](../../docs/CANONICAL_AUTHORITY_PATH_CLOSURE_BENCHMARK_V0_1.md).


## Information-flow lifecycle benchmark

The existing read-only benchmark job also runs the standard-library source-to-sink information-flow checker and mutation tests. This supplement consumes the AUTH-CORE-002 #503 origin/intent manifest contract and tests taint/source lineage through summaries, embeddings, memory and child-agent handoffs. It verifies exact observed sink-parameter digests, distinguishes policy-oracle decisions from system admission, and includes a scoped declassification positive control. It does not create another provenance ledger, DLP product, memory store or runtime flow-control service. See [IFL v0.1](../../docs/INFORMATION_FLOW_LIFECYCLE_BENCHMARK_V0_1.md).


## Typed information-flow reference model

\`src/information_flow.rs\` adds a dependency-free typed evaluator for source/derived data lineage, sensitivity and integrity propagation, exact declassification receipts, source-to-sink parameter rules, action-origin scope, and independent sink-observation matching. Its 9 authored unit tests cover the core semantics. It does not parse the JSON corpus or prove runtime flow control; the Python fixture checker and Rust typed tests are independent partial implementations, not yet a full differential replay.
