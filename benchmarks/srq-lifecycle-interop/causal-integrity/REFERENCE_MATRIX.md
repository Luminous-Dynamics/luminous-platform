# Causal reference-target matrix v1

**Research conformance benchmark only; never release qualification.** This extends the causal-integrity corpus along a different state-space axis: for every declared lifecycle reference, try every event ID present in that candidate, plus a dangling ID, an empty string, and null.

## Frozen oracle

Fourteen causal edges are enumerated across successful-promotion, terminal-failure, and failure-then-recovery paths. The target record kind for each field is frozen in both the protocol and an independent closed edge inventory in the runner. The runner refuses protocol changes unless the protocol, oracle inventory, and target metrics agree.

The domain is 143 candidate-reference states:
- 6 success-path references × 11 candidate values = 66 states.
- 3 failure-path references × 9 candidate values = 27 states.
- 5 recovery-path references × 10 candidate values = 50 states.

Expected outcome: exactly 14 accepts (the correct predecessor for each edge) and 129 rejects. The matrix requires zero false accepts, false rejects, unexpected errors, or release escalations. Each semantic mutation recomputes the unkeyed event-chain hashes, so a wrong reference cannot be rejected only because the hash chain became stale.

## Run

From the repository root:

    python -m unittest discover -s benchmarks/srq-lifecycle-interop/causal-integrity -p 'test_*.py' -v
    python benchmarks/srq-lifecycle-interop/causal-integrity/causal_reference_matrix.py --output /tmp/srq-reference-matrix.json

The report binds the exact commit, the protocol raw-byte SHA-256, the oracle script digest, and the parent checker digest and stores all per-case candidates/outcomes. The protocol digest is also pinned in the runner source.

## Limits

This is exhaustive only over the declared reference fields and candidate values; it does not prove authenticated evidence, signatures, freshness, actual target state, crash consistency, or production recovery. The event chain uses unkeyed hashes, so a fully rewritten chain can be recomputed. A PASS means only that the declared synthetic reference contract matched on this exact source revision.
