# Commit Frontier Fencing and Late-Arrival Resistance Benchmark v0.1

**Status:** synthetic resource-fence contract only. No resource owner, lock service, database or distributed transaction has been exercised.

## Purpose

CFF is the executable supplement for [issue #529](https://github.com/Luminous-Dynamics/luminous-platform/issues/529). It separates a frontier observation from the resource owner's enforcement decision.

A valid frontier certificate can report that earlier work is complete; its witness must also be qualified against OIC's critical seven failure domains (process/runtime, event stream, signing root, credential root, clock authority, storage backend and control plane), with explicit subject, resource-owner and witness identities. that does not by itself prevent an old worker from resuming later. The resource owner must atomically compare the presented generation, branch/transaction binding, policy epoch, resource version and frontier identity while applying the effect.

## Three-layer contract

1. **Observation watermark:** carries source/owner revision, completeness, allowed lateness, source-closure policy and independently qualified witness.
2. **Commit-time admission:** binds the operation to the exact frontier certificate, owner revision, resource, transaction, branch, grant, policy epoch and argument digest.
3. **Resource-side fence:** carries a monotone owner-issued generation. The resource rejects stale tokens at the same linearization boundary that commits the write.

A local lease check followed by an unguarded write is a TOCTOU race. A stale replica read is not owner authority. A restored client does not lower the target's generation. An open source is safe only when the owner contract explicitly uses target fencing to reject old epochs.

## Frozen 28-case panel

The suite contains 7 positive, 18 negative and 5 indeterminate controls. It covers current generation; stale-token rejection and acceptance; delayed lower-epoch writes; takeover between check and write; multiple writes under one active grant; cross-resource and cross-branch token reuse; stale policy; high-water rollback; stale frontier replica; allowed late-data policy with target fencing, a real late arrival while its source remains open, unknown lateness policy; stale resource version; missing owner/high-water evidence; independent resource owners; successor generation; false rejection; a frontier witness sharing the resource owner's control-plane domain; and an unknown witness domain.

## Metrics

The evaluator reports stale-token acceptance, late-epoch acceptance, wrong transaction/resource binding, policy/version mismatches, stale-frontier acceptance, TOCTOU races, high-water regression, failure of atomic fencing, owner revision/high-water mismatch, false rejection and target-side effect count separately. No composite score.

## Research targets (not results)

- zero stale-generation and late-epoch effects accepted;
- zero check/write TOCTOU races;
- 100% target high-water monotonicity across restart/restore/failover;
- 100% valid same-grant positive controls accepted;
- zero cross-resource, branch, transaction or policy reuse;
- zero stale replica/frontier acceptance;
- zero false global blocking of disjoint resources;
- no qualified decision when owner or independent high-water evidence is unknown.

## Comparative plan

Compare advisory frontiers only, frontiers plus client-side token checks, and frontiers plus resource-enforced monotone tokens with atomic CAS. Match owner topology and fault schedules. Include Beam watermark/late-arrival controls, ZooKeeper-style stale-read/takeover conditions, Spanner-like commit-time ordering, and Flink-style idempotent/transactional sink controls. Measure p50/p95 commit latency and throughput alongside correctness.

A superiority claim requires matched owner consistency, replication topology, transaction rate, fault schedule, versions, latency budget and independent target-side effect ledger.

## Claim ceiling

A passing synthetic checker validates only this contract. Stale-writer resistance requires a disposable instrumented resource owner that atomically checks a monotone token while applying writes, plus stale worker, takeover, read-replica, restart and rollback fault injection with independent target-side observations.