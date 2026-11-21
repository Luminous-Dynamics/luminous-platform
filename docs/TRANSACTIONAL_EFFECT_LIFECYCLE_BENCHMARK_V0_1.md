# Transactional Effect Lifecycle and Recovery Benchmark v0.1

**Status:** synthetic fault/lifecycle contract only. No service was contacted, no fault was injected and no model/agent execution is claimed.

## Purpose

TELR is the first executable supplement for [issue #525](https://github.com/Luminous-Dynamics/luminous-platform/issues/525). It tests the boundary between what the target committed and what the agent/harness believes happened.

It consumes existing CAPC effect identities, endpoint trace/oracle semantics, TWEP authority lifecycle, Mycelix consume-once grants, OIC/CFI observer qualification and Symthaea ASSURE evidence currentness. It does not introduce a second journal, transaction manager, idempotency gateway or effect oracle.

## Core model

Every operation has a stable semantic intent identity, CAPC effect identity, argument digest and a sequence of attempt identities. The target-side ledger records externally committed effects and compensation events. The client-side ledger records which commits the agent/harness knows about. The independent target ledger is authoritative for what happened; local completion is not proof.

Target-side idempotency keys are bound to exact arguments. A key reused with different arguments is a conflict. A new key after restore is a new operation, not a retry. A compensation event is a distinct target effect linked to its original commit; it never erases that original.

## Lifecycle

Intent → stable operation identity → authority reservation → dispatch → target commit/no commit → acknowledgement/loss → reconciliation → terminal disposition → evidence seal.

Checkpoint restore only restores local computation. It cannot roll back an external side effect or resurrect a consumed nonce/approval. Unknown outcome blocks new mutation until authoritative reconciliation or a documented, exact same-key target idempotency contract permits redelivery.

## Frozen 20-case panel

The panel covers pre-dispatch no-effect and committed controls; target commit with lost acknowledgement; unknown dispatch and late-commit uncertainty; same-key/same-argument redelivery; duplicate target commits; same-key/different-argument conflict; checkpoint restore with changed operation key and semantic intent replay; consumed nonce resurrection; partial-batch reconciliation; orphaned target commit after local evidence failure; client-ledger false commit claim; valid compensation and invalid original link; blind new-key retry under unknown outcome; evidence-integrity failure; reconciliation claiming no commit despite a target-side commit; and duplicate compensation delivery.

Distribution: seven positive, nine negative and four indeterminate controls. These are deterministic synthetic contract cases, not independent statistical samples.

## Reconciliation semantics

- Committed: reconciliation commit ids exactly match the independent target ledger.
- Not committed: target ledger has no effect and a complete read/status path is available after known consistency and in-flight bounds.
- Partial: a batch's committed subset is explicitly reconciled item-by-item.
- Unknown: target observation incomplete, no read-back, no known in-flight bound, or evidence does not resolve the state.

An empty early read is not no-effect proof if an operation may still be in flight. No verification-only policy can guarantee exactly-once under an unbounded late-commit model.

## Metrics

Report duplicate committed effects, key/argument conflicts, semantic replay, unsafe retries, authority resurrection, orphaned commits, client/target ledger conflicts, compensation-link violations, reconciliation state and final disposition separately. Also report throughput, latency, read amplification, durable-write overhead, recovery time, unresolved availability and benign task completion. No composite score.

## Research targets (not results)

- Zero duplicate target effects per stable operation in frozen duplicate/replay controls.
- Zero new-key semantic replays after checkpoint restore.
- 100% consumed authority remains consumed across restore.
- 100% partial-batch element outcomes match the independent target ledger.
- Zero false no-effect/success claims under unknown or incomplete observation.
- Zero new mutation while a previous outcome is unresolved, except exact same-key/same-argument target-side idempotent redelivery.
- 100% compensation events remain separate linked entries; original effects are never erased.
- Deterministic expected outcomes for all frozen controls.

## Comparative plan

- Reproduce a compatible subset of LIMBO's six service types and twelve fault modes, matching models, tool contracts, read-back consistency and fault timing.
- Reproduce crash-after-reservation, crash-before-effect, crash-after-effect-before-completion, response loss/delay, replay and evidence-write failures from *Idempotency Is Not Recovery*.
- Reproduce ACRFence Action Replay and Authority Resurrection patterns using changed request ids after restore.
- Retain ToolSandbox stateful-task utility outcomes, then add independent target-ledger grading without replacing its task oracle.
- Compare client-only verify/retry, harness ledger, target idempotency key, durable intent/outbox, reconciliation and full layered recovery through ablations, identical model proposals and paired fault assignments.

A superiority claim requires matched models, fault timing, service contracts, consistency bounds, independent target ledger, sample sizes and latency/throughput.

## Claim ceiling

A passing checker qualifies only the synthetic lifecycle semantics. At-most-once/exactly-once/recovery claims require a disposable service with target-side idempotency/status, controlled fault injection, independently captured target effects and cross-platform reconciliation.