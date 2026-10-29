# SRQ bounded linearizability oracle v1

## Why a different checker

The serialized history oracle checks event integrity and receipt/effect binding, but requires a normalized total event order. Concurrency claims require a different question: does there exist a legal sequential execution consistent with operation overlap intervals?

This checker accepts invocation/completion intervals and searches for a sequential witness under a narrow idempotent-sink model. It does not assume the exporter order is the execution order. Candidate orderings must preserve real-time precedence for non-overlapping operations and satisfy the abstract contract.

## Run

    python3 -m unittest discover -s benchmarks/srq-lifecycle-interop/guarantee-vector/linearizability-oracle -p 'test_*.py' -v
    python3 benchmarks/srq-lifecycle-interop/guarantee-vector/linearizability-oracle/linearizability_oracle.py benchmarks/srq-lifecycle-interop/guarantee-vector/linearizability-oracle/fixtures/valid-concurrent-same-key.json

LINEARIZABLE supplies one witness. NON_LINEARIZABLE means bounded exhaustive search found none. UNKNOWN_LIMIT is inconclusive. INVALID_HISTORY means input is malformed. This first implementation is a correctness-oriented reference, not a speed claim.

## Clock semantics

All intervals use one observer's monotonic clock. If A returns before B is invoked (A.end_ns < B.start_ns), A must precede B. Overlapping intervals may be ordered either way when the abstract model permits it. Equal endpoints are treated as ambiguous due to clock resolution.

Do not use raw timestamps from unsynchronized hosts. Distributed adapters need a justified common-clock mapping with uncertainty or a causal/happens-before representation.

## Abstract contract

The state binds an idempotency key to request/effect/policy digests. Committed retries must return stable effect and receipt identifiers. A different intent under the same key can return key_conflict but cannot commit. An unknown result may mean the request did not reach the sink, reserved the key without committing, or committed with identifiers not visible to the caller; a retry may resolve that ambiguity.

Authorization, retention expiry, namespace compaction, transactions and anchor state are outside this version. It checks one finite history only.

## Differential test matrix

- Concurrent same-key retries with a stable effect/receipt pair: linearizable.
- Overlapping same-key commits with different effect/receipt IDs: non-linearizable.
- Key conflict that returns before the alleged key-creating operation starts: non-linearizable.
- Unknown result followed by a stable committed retry: linearizable.
- Search budget exhaustion: inconclusive, not failure.

## Industry baseline

Linearizability is an established consistency model. Jepsen formalizes it as a legal sequential history consistent with real-time ordering; Porcupine is a mature model-based checker using backtracking and optimizations such as caching and partitioning. This initial independent reference is bounded and deliberately unoptimized. It must be validated against independent histories and compared with a mature checker before performance claims.

- Jepsen model: https://jepsen.io/consistency/models/linearizable
- Porcupine (Go): https://github.com/anishathalye/porcupine
- Porcupine Rust port: https://github.com/erwin-kok/porcupine-rs

The opportunity is not to claim SRQ beats these tools today. It is to bind a checker to SRQ-specific idempotency, receipt and unknown-outcome behavior, emit witness orderings, and then check real fault-injected sink histories.

## Proposed benchmarks

1. Correctness: 100% agreement on an independently reviewed finite corpus; differential agreement with Porcupine or another independently developed checker where the contracts overlap.
2. Exhaustive small histories: enumerate all short histories for 2–4 operations across key intents, outcome variants and overlap patterns; compare with a separately implemented specification.
3. Mutation adequacy: each seeded bad history must yield the intended contract failure, not merely a generic FAIL.
4. Scalability: measure wall time, explored states, peak memory and UNKNOWN_LIMIT at 8/16/32/64 operations only after correctness checks.
5. Real sink: inject crash before commit, after commit before response, concurrent same-key calls, stale fencing, restore and rollback. Retain raw intervals and exact-head hashes.

Safety, liveness and performance remain separate. The checker does not certify a real adapter. Qualification remains BLOCKED, claim ceiling none.
