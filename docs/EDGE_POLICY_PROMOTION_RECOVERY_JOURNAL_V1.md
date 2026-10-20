# Luminous Edge policy promotion and crash recovery v1

**Status:** proposed implementation and software tests; not yet qualified against physical access points, enterprise firewalls, or controller fleets.

**Claim ceiling:** this journal makes policy-change states durable and fail-closed. It is not an authenticated device channel, a trusted clock, signed remote attestation, or an atomic transaction with external policy application.

## Paradigm shift: treat policy change as a recoverable protocol

A device command is not a policy outcome. A reported success is not readback. Readback is not an independent service probe. A probe accepted by a freshness verifier is not yet a committed promotion. The controller must keep those claims distinct.

The lifecycle is:

`Prepared → ApplyIntent → Applied → Verified → Promoted`

Rollback has its own explicit path:

`ApplyIntent | Applied | Verified → RollbackIntent → RolledBack`

- **Prepared:** immutable target, generation, baseline digest, and candidate digest are durably recorded.
- **ApplyIntent:** the intent is committed before the external apply command. A crash here leaves the real device state uncertain.
- **Applied:** a live effective-policy readback exactly matches the candidate digest. A controller acknowledgement cannot make this transition.
- **Verified:** a caller that has completed conformance reconciliation, freshness validation, and durable nonce consumption records an independent probe bound to the candidate digest and receipt digest.
- **Promoted:** a still-live, process-local permit is presented, evidence has not expired, and a second effective-policy readback still matches the candidate digest.
- **RollbackIntent:** rollback is durably requested before issuing external rollback.
- **RolledBack:** a live readback exactly matches the original baseline digest.

Only `Promoted` and `RolledBack` are terminal. A target may have only one non-terminal operation, and a policy generation cannot be reused for that target. A new operation must use a strictly higher generation than any previous attempt.

## Crash and restart behavior

Opening the journal invalidates every persisted `Verified` state, clears its evidence fields, and moves it back to `Applied`. The old process-local promotion permit is not serializable and cannot be recreated from the journal. The restarted verifier must obtain a new challenge and perform a new independent probe before it can mint a fresh permit.

This intentionally prefers safe re-verification over liveness. It handles the important crash windows without pretending that two independent durable systems and a real network device share one atomic transaction:

| Crash point | Durable state | Recovery rule |
|---|---|---|
| Before prepare commit | No operation recorded | Do not issue external actuation |
| After `Prepared`, before apply intent | `Prepared` | Explicitly apply or abandon according to controller policy |
| After `ApplyIntent`, before/after device command | `ApplyIntent` | Read back effective policy before retrying; do not blindly replay a non-idempotent command |
| After readback, before `Applied` commit | `ApplyIntent` | Inspect device state, then record candidate only if its exact digest is present |
| After durable nonce consumption, before `Verified` journal commit | `Applied` | The old nonce remains consumed; issue a fresh challenge and repeat the independent probe |
| After `Verified`, before promotion commit | Reset to `Applied` at next open | Discard old evidence and require a fresh challenge/probe |
| During rollback | `RollbackIntent` | Inspect actual state and enforce/read back the baseline; never label rollback complete from command acknowledgement |

If actual state is neither the expected candidate nor the approved baseline, this journal does not guess which policy is safe. The deployment-specific recovery controller must fail closed, halt further rollout, and resolve the discrepancy.

## API use contract

The Rust module is `tools/edge-policy-observation-reconciler/src/promotion_journal.rs`.

1. Call `prepare` and commit `ApplyIntent` before invoking the external controller.
2. Apply the candidate, independently read the effective ruleset, and call `record_applied` only when its exact digest equals the candidate.
3. Run the existing freshness/conformance verifier, call `verify_and_consume_durable` from the replay-ledger module, and only then call `record_verified`. The journal method cannot prove its caller followed that order; a production adapter must encapsulate the sequence and make bypassing it difficult.
4. Call `promote` using the returned non-cloneable process-local permit, the current time, and a second live policy readback.
5. On any mismatch or error, do not promote. Persist rollback intent before external rollback; record completion only after the baseline digest reads back exactly.
6. After a crash, use `list_recoverable` as a diagnostic for the required recovery action; the list itself does not actuate devices.

The replay nonce ledger and promotion journal are currently separate durable databases. This is deliberate in v1 and means the gap is handled by safe re-probing, not by claiming cross-database atomicity. A subsequent integration layer should own both paths, encapsulate the critical sequence, and include crash-injection tests at each boundary. The external device's apply/readback/probe operations remain non-atomic and must use device-specific idempotency, locking, and reconciliation semantics.

## Security and operational limits

- SHA-256 binds exact bytes/digests, but does not authenticate who produced them.
- The API trusts that the caller's reported observation came from a genuinely independent probe and that the evidence's expiry came from an already-validated challenge envelope.
- The journal assumes its process, binary, database, parent directory, and storage are in the trusted control plane. It rejects symlink paths and database files that grant group/other access on Unix; use a trusted directory and durable filesystem.
- Session binding uses OS randomness only to distinguish process sessions; it is not an authentication credential.
- A local attacker able to replace the verifier, database, clock, or device adapter is outside this module's protection boundary.
- WAL plus SQLite FULL synchronous mode helps with process/system recovery, but does not override dishonest storage hardware, filesystem bugs, or all power-loss behavior.
- Generation monotonicity is journal-local. A fleet controller still needs a single-writer/lease or consensus rule and a device-side compare-and-swap/version check to prevent concurrent external writers.
- The second readback and promotion commit are still a time-of-check/time-of-use boundary. Device-specific locks or revisions are required to close it.

## Benchmarks and acceptance targets

Targets below are proposed qualification gates, not achieved results.

| Dimension | Suggested method | Initial acceptance target |
|---|---|---|
| State-machine correctness | Exhaustive legal/illegal transition table; negative tests for every skipped transition | Zero illegal transitions accepted |
| Crash recovery | Kill process after every durable transition and at each external actuation boundary; reopen journal and inspect device state | Zero stale-evidence promotions; recovery always explicit |
| Evidence freshness | Generate stale, future-dated, wrong-digest, wrong-target, expired, and replayed receipts | Zero invalid receipts promoted; every negative test has a valid positive control |
| Generation/control concurrency | Concurrent prepare attempts for the same target and generation from independent processes | At most one non-terminal operation per target; no generation reuse |
| Rollback correctness | Inject apply failure and rollback failure; compare effective digest, then run a positive control probe | No rollback success without exact baseline readback and control-path success |
| Reproducibility | At least 30 randomized paired trials per qualified device/firmware/policy tuple | Report all failures and inconclusive trials; no denominator omissions |
| Replay stress | At least 100,000 unique challenges mixed with duplicate/replayed/stale challenges | Zero invalid accepted; report throughput and p50/p95/p99 latency |
| Scale | 100, 1,000, 10,000, and 100,000 targets/rules where the vendor adapter supports it | Report reconciliation and promotion latency, CPU, memory, DB growth, lock waits, and operator intervention |
| Durability | Process-kill and restart around prepare/intent/commit; storage fault injection on a disposable test volume | No transition reported committed before durable success; corruption fails closed |

Report policy-controller time separately from end-to-end time. Measure p50/p95/p99 from intent commit to independently observed service outcome, and separately report command duration, effective-policy readback, probe duration, and rollback recovery. A command's single-run runtime is not a rollout SLA.

## Competitive comparison

- **Differential configuration analysis** such as Batfish can compare reachability and policy behavior between network snapshots. The opportunity here is to join planned/differential semantics to *observed* effective policy, fresh challenge-bound evidence, durable replay state, and recoverable promotion/rollback.
- **Stateful firewall vendors** may rematch existing sessions after policy changes. That is vendor- and configuration-dependent; Luminous should test explicit new-flow, existing-flow, explicit-revoke, and restoration semantics rather than assume policy edits terminate sessions.
- **Attestation and freshness architectures** already treat freshness as a first-class claim. This journal builds on that principle but does not substitute for signed evidence or remote attestation.
- **Network performance testing** must separate configuration correctness from throughput. Use a published method such as RFC 6349 only after lower-layer path integrity and baseline parameters have been established.

The differentiated product goal is not “another ruleset editor.” It is a portable change protocol whose promotion claim is attached to a specific generation, exact effective policy, fresh independent observation, durable replay decision, and tested recovery path. Competitive superiority remains a hypothesis until like-for-like vendor tests quantify correctness, rollout impact, recovery time, operational toil, and scale.

## Current implementation evidence

Unit tests in this module exercise illegal transitions, candidate-digest checks, freshness expiry, one-use process-bound permits, restart invalidation, recovery-action classification, baseline-exact rollback, active-target exclusion, and generation reuse rejection.

These tests should be paired with CI on the exact head and then expanded into real process-kill/fault-injection qualification. This is not yet physical AP/firewall or multi-device fleet evidence.
