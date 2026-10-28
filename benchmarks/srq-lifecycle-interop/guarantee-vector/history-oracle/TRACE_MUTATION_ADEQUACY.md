# SRQ trace-mutation adequacy v1

## Paradigm shift: mutation-test the checker, not only the system

PR #135 introduced mutations to evaluator source code and asks whether the named Rust regression test fails. PR #137 introduced an independent history oracle that checks observed sink traces. This increment attacks the next hidden assumption: **does the oracle detect the kinds of bad histories we say it detects?**

The harness starts from one valid retry history, changes one property in an isolated in-memory copy, and requires the oracle to emit the expected stable violation code and invariant ID. An invalid trace, a generic FAIL, or an unrelated invariant is not counted as detection. The executable corpus and versioned manifest must agree exactly on mutation IDs, ordering, expected codes, and invariant IDs.

## Run

```sh
python3 -m unittest discover -s benchmarks/srq-lifecycle-interop/guarantee-vector/history-oracle -p 'test_*.py' -v
python3 benchmarks/srq-lifecycle-interop/guarantee-vector/history-oracle/trace_mutation_qualification.py --output /tmp/srq-trace-mutation-report.json
```

## Current corpus

The initial corpus has 23 deliberate trace faults across all seven frozen invariant families: changed intent under a reused key, duplicate effects and IDs, missing/mismatched receipts, wrong sink bindings, false response claims, stale or mismatched fencing, non-contiguous histories, orphan events, and duplicate terminal responses.

Acceptance is 23/23 expected detections, zero survivors, zero invalid mutations, and one valid baseline trace with zero violations. A detected case must include both the exact expected code and invariant ID. The report records the event SHA, checker/protocol/manifest/harness hashes, workflow-event SHA, exact checked-out source SHA, and PR source-head SHA.

## Explicit limits

This is a finite test of *checker sensitivity to selected trace faults*. It does not mutate the checker source, prove the checker complete, authenticate the event recorder, prove that an adapter emitted every real operation, establish transaction durability, establish distributed linearizability, or qualify production. One fixture can also miss interactions not represented by this trace; expanded generated traces and real-sink histories are required next.

## Industry comparison

Jepsen's public analyses describe a complementary method: generate operations against real systems, inject failures, capture histories, and check them against consistency models. Jepsen explicitly notes that such experiments can find errors but cannot prove their absence, and that fault-driven correctness testing is not by itself a performance benchmark. SRQ should combine that real-system method with this smaller deterministic mutation corpus, not claim to replace it: https://jepsen.io/analyses and https://jepsen.io/analyses/ethics.

AWS's idempotent-API guidance is the baseline for stable intent per client token and rejection of same-token/different-parameters reuse: https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/. TUF's rollback/freeze checks motivate the separate persistence/epoch test family still to be run against the eventual anchor adapter: https://theupdateframework.github.io/specification/latest/.

## Next gate

Generate property-based traces, minimize failing histories, then connect an actual persistent sink exporter. Inject crash before commit, after effect/receipt commit but before reply, concurrent same-key calls, stale workers, snapshot restore, and anchor rollback. Preserve raw trace bytes and independent checker report. Report safety violations separately from completion rate, recovery time, and latency. Qualification remains **BLOCKED** until the real adapter evidence exists.
