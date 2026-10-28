# SRQ external side-effect history checker (research prototype)

This tool is the next assurance layer after state-matrix tests and mutation testing: it checks a captured request history against the **sink's own durable journal**, rather than trusting the decision evaluator's summary alone.

## Status and claim ceiling

This is a deterministic Python reference checker for benchmark/qualification use, not production code. Its fixtures are synthetic. A `PASS` means only that the supplied trace satisfies the listed checks. It does not authenticate the trace, prove the adapter supplied a complete history, independently establish disk durability or transaction atomicity, measure retention expiry, or prove full linearizability. The qualification state remains `BLOCKED`, and the claim ceiling remains `none` until a real adapter emits independently reviewable evidence and the fault-injection suite runs against it.

## Input contract

`history-protocol-v1.json` defines `srq-side-effect-history-v1`. Input is a JSON object with:

- `trace_id`, `sink_id`, and `fencing_required`;
- `operations`: observed request outcomes (`success`, `rejected`, or `unknown`). Every success binds an operation to its transaction, effect ID, and receipt ID;
- `transactions`: one sink-journal commit record that binds a key + request digest to one effect ID and one receipt ID, with a unique durable `commit_seq` and optional fencing token;
- `fence_events`: durable fencing advances with `event_seq` and token.

All `commit_seq` and `event_seq` values must share one sink-controlled durable sequence domain. The commit record must describe the atomic effect/receipt transaction; do not manufacture it by joining non-atomic logs after the fact. An adapter must document where each field originates and how truncation, replication lag, backups, and restore are handled.

Run the fixture suite:

```sh
python3 -m unittest discover -s benchmarks/srq-lifecycle-interop/guarantee-vector -p 'test_history_checker.py' -v
```

Run the checker against an adapter-exported trace:

```sh
python3 benchmarks/srq-lifecycle-interop/guarantee-vector/history_checker.py trace.json --output report.json
```

Exit code `0` means the trace passed these checks; non-zero means invalid input or violations. Reports deliberately retain `qualification_status: BLOCKED` and `claim_ceiling: none` in both cases.

## What it checks

The checker rejects unbound successful responses, effect/receipt identity mismatches, more than one committed transaction per idempotency key, changed intent accepted under a used key, divergent successful retry identities, partial effect/receipt records, and stale/non-monotonic/reused fencing tokens. An `unknown` operation is not automatically a failure: a timeout can happen after commit; it must be reconciled against the durable journal before claiming success or safe retry.

## Paradigm shift and benchmark ladder

The core shift is **evidence from the sink, not self-attestation by the policy layer**. An assurance token should be bound to a specific sink identity and contract, and the acceptance decision should be checked against durable side-effect history.

1. **Local oracle adequacy:** one positive fixture plus ten negative controls; require zero false accepts and zero false rejects.
2. **Generated trace adequacy:** at least 10,000 seeded generated histories, minimized counterexamples, and a reviewed mutation corpus of 20–40 realistic faults. Track checker soundness and rejection reasons independently.
3. **Real adapter:** choose a concrete transactional sink; document the exact atomic commit boundary and extract journal records. Start with one durable backend rather than claiming backend portability.
4. **Fault injection:** at least 100 crash/restart trials per durable boundary, then 1,000 restore/replay/rollback trials per supported adapter. Include lost responses after commit, duplicate submissions, concurrent same-key requests, changed-intent key reuse, stale writers, and restore from an old snapshot.
5. **Concurrency and cost:** run 1/8/64/256 concurrent submitters and publish p50/p95/p99, throughput, lock contention, journal bytes/write amplification, recovery time, and blocking rate. Compare only implementations with equivalent durability/fencing semantics.

A future full history checker should model operation invocation/completion intervals and validate linearizability independently, with concurrent histories and failure injection. This version intentionally does not claim that. Established guidance emphasizes both stable idempotency identity and atomic recording of the token with the side effect; freshness/epoch signals also need replay, delay, and reordering protections. Those are external design baselines, not proof of SRQ conformance.
