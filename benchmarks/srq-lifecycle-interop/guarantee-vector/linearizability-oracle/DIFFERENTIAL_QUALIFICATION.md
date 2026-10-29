# SRQ linearizability differential qualification v1

## Why this exists

A custom checker can agree with hand-written examples and still contain a bug in candidate ordering, memoization, transitions or ambiguous-outcome handling. This layer compares the optimized backtracking checker with a deliberately simple reference that enumerates every candidate permutation and independently applies the sequential contract.

The reference shares only input decoding with the optimized checker. It does not call the optimized search routine or private state helpers. Every linearizable answer must include a witness that the reference independently accepts. Search-limit exhaustion or malformed corpus records fail the gate rather than counting as negative histories.

## Frozen corpus

- 72 operation shapes: 2 keys × 2 intent values × 3 interval shapes × 6 result/identifier shapes.
- Exhaustively enumerate every ordered two-operation pair: 72 squared = 5,184.
- Add 512 deterministic three-operation histories and 128 deterministic four-operation histories.
- Total: 5,824 histories per run.
- Pin the canonical corpus SHA-256 in the versioned protocol and fail if regeneration differs; also retain disagreement examples with raw histories and digests, source hashes and exact checkout/head provenance.

The three- and four-operation tiers are deterministic samples, not exhaustive. The two-operation tier is exhaustive over the declared finite shape domain.

## Run

    python3 -m unittest discover -s benchmarks/srq-lifecycle-interop/guarantee-vector/linearizability-oracle -p 'test_*.py' -v
    python3 benchmarks/srq-lifecycle-interop/guarantee-vector/linearizability-oracle/differential_qualification.py --output /tmp/srq-linearizability-differential.json

Acceptance is exact agreement across all 5,824 histories, no invalid inputs, no unexpected search limits, no optimized witness rejected by the reference, and a canonical corpus digest equal to the protocol's pinned value. The hash prevents a silent generator change from redefining the experiment.

## Industry relationship

This is a correctness/differential gate, not a replacement for a mature checker. Porcupine-RS uses WGL/Lowe-style backtracking, caching and optional partitioning; Jepsen offers real-system orchestration, fault injection, history recording and model-based checking. The differential reference is an assurance step before comparing runtime or adopting optimizations.

- Porcupine-RS algorithm: https://github.com/erwin-kok/porcupine-rs
- Jepsen history generation and checking: https://github.com/jepsen-io/jepsen
- Jepsen linearizability model: https://jepsen.io/consistency/models/linearizable

## Next performance gate

After zero correctness disagreements, compare SRQ to Porcupine-RS on the same abstract model and equivalent histories. Record history size, concurrency width, model/state cardinality, wall time, peak memory and timeout rate. Do not announce superiority unless corpus, models, hardware and stopping rules are equivalent.

## Limits

This finite differential corpus cannot prove either checker or the abstract contract correct. The three- and four-operation histories are sampled. The clock must be from a single monotonic observer. No real sink, fault injection, durability, or production qualification is claimed. Qualification remains BLOCKED.
