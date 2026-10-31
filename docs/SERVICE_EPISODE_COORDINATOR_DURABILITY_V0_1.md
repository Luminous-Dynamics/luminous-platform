# Service-Episode Coordinator Durability Contract v0.1

**Status:** research contract / synthetic evaluator specification.  
**Evidence ceiling:** E0/E1 only. No durable runtime or production crash guarantee is implemented by this document.

## 1. Scope and ownership

The recovery unit is a cross-system **service episode**. The Platform coordinator may persist episode identity, plan digest, step state, authorization references, intent/receipt references, fencing state and reconciliation metadata.

**Platform owns the service-episode coordinator history. Nixward owns the host transaction.** When a saga step delegates a host mutation to Nixward, Platform stores the Nixward transaction ID, exact ChangePlan digest, attempt identity and receipt/evidence digests as references. It does not create a second host journal, worker identity, host interlock, authorization verifier or host recovery executor.

The coordinator store is responsible for the durable history required to reconstruct *coordination state*. The independent participant ledger is responsible for reconciling whether a specific remote attempt was accepted or committed. The service oracle is responsible for independently observing the final service postcondition. These are three different evidence sources.

## 2. The dispatch commit barrier

A connector operation with external side effects may be invoked only after all of the following are true:

1. The exact plan, step, target, operation, request digest, grant reference, unique attempt ID and stable logical idempotency key are bound.
2. The coordinator has appended a dispatch-intent event with the current episode revision and fencing token.
3. The selected journal backend has returned its documented durable-commit acknowledgement (for a database, the configured transaction commit semantics; for a file WAL, the relevant data and metadata sync contract).
4. The coordinator still owns the episode under the current durable fence.
5. The action's authority remains current and exact at the point of dispatch.

A failed or lost storage acknowledgement is ambiguous. Read back the event/transaction and prove its state before proceeding. Never infer that a failed write definitely did not commit.

A connector call that may have crossed its admission boundary and then times out becomes `outcome_unknown`. Recovery must query the same attempt identity against an authoritative participant-owned ledger. It may not create a new attempt merely because the coordinator process restarted, the current state looks unchanged, or an eventually-consistent query returns no row.

## 3. Replay and snapshot rules

The journal is append-only within an episode and carries a monotonic sequence/revision. Each event binds its predecessor (or equivalent authenticated journal prefix), event type, schema version, plan/step/attempt identifiers and payload digest.

Replay is deterministic over a valid journal and a pinned reducer/schema version. Unknown versions, sequence gaps, conflicting duplicate events, checksum/hash-chain failures and untrusted snapshots fail closed. A snapshot is only an acceleration structure: its last-applied sequence **and** digest must match the journal prefix. A stale snapshot can be discarded and replayed from a valid log; a mismatched digest is not silently accepted.

Do not treat a clone of an in-memory object as a restart test. A durable qualification must terminate a real subprocess, close/reopen the chosen store and rehydrate state from bytes committed under the documented durability settings. Crash tests must include ambiguous storage acknowledgements, partial tails, disk-full, sync/commit errors and publication/rotation failure.

## 4. Fencing and zombie coordinators

A restarted or failover coordinator must acquire a new durable ownership/fencing token (or equivalent transactional compare-and-swap). Every state-changing event and dispatch barrier must prove the current token. A process that is merely alive is not necessarily the owner; an old worker resuming after a partition must fail its next fenced write and stop.

Time-based leases alone are not sufficient evidence of exclusive ownership because clocks, pauses and partitions can make a previous owner appear expired while it is still executing. The store's fencing transaction is the authority.

## 5. Compensation and cancellation

Cancellation prevents undispatched future work; it does not reverse a dispatched effect. Any in-flight step remains unknown until reconciled.

Compensation is another externally visible operation: it needs a separate exact grant, distinct attempt identity and its own durable dispatch intent. A restart cannot infer that compensation ran or safely issue it again from the absence of a response.

## 6. Fault matrix and independent oracle

`benchmarks/service-episode-durability-v0.1/crash-matrix.json` defines 24 evaluator-only synthetic fault cases across eight families. It is not a prompt, SUT task manifest, runtime policy or oracle input. The schema and dependency-free structural checker freeze the inventory and prevent external replay from being allowed.

Every real harness run must use a participant-owned side-effect counter or equivalent authoritative record independent of the coordinator log and SUT-reported state. The oracle counts external effects by stable logical operation and exact attempt. Service health is a separate oracle from participant effect count.

## 7. Benchmark targets and comparison

The matrix separates hard safety gates from performance. Hard gates are zero lost acknowledged events, zero duplicate effects, zero unauthorized dispatches, zero automatic replay under unknown effect, zero writes from stale owners, and 100% deterministic replay of valid histories with fail-closed handling of malformed histories.

Initial hypotheses—not measured results—are RPO 0 for acknowledged commits under the declared store failure model, same-node restart RTO p95 at or below 5 seconds for the reference lab workload, p95 episode-latency overhead at or below 10%, and no more than one human adjudication per 1,000 deterministic lab episodes. Publish p50/p95/p99, hardware, storage engine, sync/transaction settings, event volume, fault schedule and confidence intervals. A safety-gate failure is disqualifying and cannot be offset by low latency.

Fair comparison arms should include a minimal Platform coordinator journal, a Temporal event-history/replay implementation, a DBOS/PostgreSQL checkpoint implementation and a non-durable in-memory control. Same tasks, permissions, participant contracts, failure schedule and independent effect oracle are required. Temporal documents durable event histories and replay after worker crashes; DBOS documents PostgreSQL-backed workflow/step checkpoints and recovery after interruption. These are relevant durable-execution comparators, not proof that arbitrary third-party side effects are exactly once. See [Temporal Event History](https://docs.temporal.io/encyclopedia/event-history), [Temporal Workflows](https://docs.temporal.io/workflows), [DBOS Architecture](https://docs.dbos.dev/architecture) and [DBOS Workflow Recovery](https://docs.dbos.dev/production/workflow-recovery).

## 8. Claim ceiling and next phase

This contract and synthetic matrix support E0/E1 design work only. A backend prototype may establish behavior only for the chosen database/filesystem, sync settings, hardware, failpoints and tested participant adapters. No universal power-loss durability, commercial-connector idempotency or production-readiness claim follows from structural validation or an in-memory fixture.
