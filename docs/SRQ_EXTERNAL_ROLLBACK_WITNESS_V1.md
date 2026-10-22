# SRQ-1 external rollback witness v1

**Status: design proposal / next qualification boundary.** This document describes a requirement; it does not claim an external witness has been implemented or qualified.

## Problem: atomicity is not rollback resistance

SQLite's transaction and integrity checks answer whether the *current database state* is internally consistent. A signed receipt can authenticate a statement about a particular state. Neither mechanism, by itself, proves that the state is the newest state this target has previously committed.

If an operator or attacker restores an old, internally valid SQLite snapshot, that snapshot can restore an old target generation and make a previously consumed one-use challenge appear unused. This is a different fault from killing a process mid-transaction or losing power during one write. Local ACID correctness cannot establish anti-rollback against restoration of the entire persistence domain.

TUF already addresses rollback of update metadata against previously trusted metadata versions. OSTree already addresses old-or-new atomic deployment transitions. SRQ should retain those strengths and add an independently anchored lifecycle checkpoint for the target's authorized policy/boot state; it should not claim to replace TUF or OSTree.

References:
- TUF 1.0.37 specification (rollback, freeze, mix-and-match and wrong-software protections): https://github.com/theupdateframework/specification/blob/master/tuf-spec.md
- TUF threats and security properties: https://theupdateframework.io/docs/security/
- OSTree atomic upgrade design: https://github.com/ostreedev/ostree/blob/main/docs/atomic-upgrades.md
- RFC 9334 RATS freshness architecture: https://www.rfc-editor.org/rfc/rfc9334.html
- SQLite WAL mode and the WAL-reset bug caveat: https://www.sqlite.org/wal.html

## Genesis and outage behavior

Provisioning a target's genesis checkpoint is itself a trust-establishment operation, not a local convenience. The reference state machine now refuses genesis initialization while the witness is marked unavailable and confirms that no target was created by the rejected attempt. A production witness must likewise fail closed during first enrollment: target identity and the genesis digest must be authenticated by a separately authorized provisioning workflow. Do not let an outage silently create a fresh sequence-zero anchor, because that would make a restored or replaced target appear newly enrolled.

## Anchor contract

Maintain a monotonic checkpoint for each target in a failure/rollback domain that is independent of the SQLite database being protected. A checkpoint binds at least:

- target identity and an explicit protocol/profile version;
- monotonic sequence number and prior checkpoint digest;
- policy generation and the exact previous/new generation transition;
- promotion decision ID, consumed challenge ID and idempotency key;
- approved artifact digest, effective policy digest, corpus digest, and immutable deployment identity;
- timestamp/freshness policy and witness identity/assurance level;
- the digest of the canonical checkpoint payload.

The witness must perform a compare-and-swap from an expected (sequence, prior digest) to exactly one next checkpoint. It must reject skipped sequence numbers, stale predecessors, conflicting use of a committed sequence, wrong target/profile, and reuse of a decision ID for different checkpoint content. A repeated request with the exact same idempotency key and exact same payload may return the original committed result. Equality must be checked against the stored full checkpoint, not merely the caller-supplied digest and sequence: the state-machine regression test deliberately reuses an unchanged digest while mutating a deployment field and requires `DecisionReuseConflict`. Cryptographic verification must additionally establish that the checkpoint digest binds the canonical payload.

A checksum or signature on a locally stored witness receipt does not make the witness independent if both receipt and monotonic state can be rolled back together. A deployment must identify the actual trust boundary: e.g., a remote append-only verifier/service with durable monotonic state, a hardware-backed monotonic facility when its security and wear characteristics are appropriate, or a quorum of independent witnesses. TPM/NV counters are not interchangeable with a remote quorum; their availability, provisioning, wear and recovery properties must be measured separately.

## Promotion protocol with recoverable ordering

Do not attempt an uncoordinated two-database write and hope both commit. Use an explicit recoverable protocol:

1. Validate the signed, target-bound promotion request and current local state. Create a durable local **prepared intent** with a unique decision/idempotency ID. A prepared intent is not an accepted promotion.
2. Submit the canonical next checkpoint to the external witness with compare-and-swap against the caller's expected prior checkpoint. The witness validates target, previous sequence/digest, freshness and idempotency, then durably advances its own monotonic state.
3. Once a valid witness receipt is returned, finalize local SQLite policy state, challenge consumption and the witness-receipt digest in one transaction.
4. On restart, read both local intent and witness state before enabling promotion or privileged operations:
   - no witness advance + prepared intent: the intent may be safely abandoned/retried only while still authorized;
   - witness advanced with matching intent: finalize idempotently or enter explicit recovery; do not create a second decision;
   - witness advanced but local state is older: classify as LOCAL_STATE_ROLLBACK_OR_INCOMPLETE_FINALIZATION, block new promotions and reconcile to the witness;
   - local state claims a newer/mismatched checkpoint: fail closed and raise an integrity incident.
5. Generate post-boot challenge only after the durable promotion and witness checkpoint agree. Reconcile the actual booted deployment observation against that same checkpoint, not only against fields copied from the promotion request.

The witness advance is the anti-rollback linearization point. After it commits, expiry of the original request must not cause an unrecorded second decision; the authorized idempotent recovery path completes or explicitly blocks that same decision. The witness must validate the authorization freshness at its own commit boundary using its defined time policy.

## Availability policy

Fail closed for new promotions when the anchor cannot be read or advanced. Do not silently fall back to a local-only counter while retaining the same claim. Whether a target is allowed to keep booting an already-installed, previously witnessed generation while offline is a separate relying-party policy. Expose separate statuses for CURRENT_AND_WITNESSED, OFFLINE_LAST_KNOWN, LOCAL_STATE_STALE, WITNESS_CONFLICT, WITNESS_UNAVAILABLE, and NOT_RUN; none of the latter four is release PASS.

The offline policy must explicitly state its maximum accepted witness age, allowed operations, revocation behavior, and how a target re-enters service after witness availability returns.

## Fault matrix / acceptance tests

Run these against the actual adapter and each witness implementation, then separately against a model:

- SQLite file restored from a known pre-commit snapshot while witness remains at sequence N+1: must detect local rollback and refuse a second promotion.
- Crash after local prepare but before witness CAS: no accepted promotion; retry is safe only with fresh authorization.
- Witness commits but response is lost: retry is idempotent and returns the same witness receipt.
- Witness commits, then process is killed before local finalize: restart discovers the committed checkpoint, finalizes the same decision or blocks visibly; no duplicate promotion.
- Local finalize succeeds but post-boot observation reports old generation/deployment: reconciliation is blocked; policy cannot call it successful merely because the SQL rows agree.
- Old witness receipt, stale predecessor, conflicting next checkpoint, wrong target, changed artifact/policy/corpus digest, duplicated decision ID and invalid signature are rejected.
- Witness database snapshot itself is restored: the witness architecture must either detect it from independent monotonic state or explicitly state that this threat is outside its guarantees.

Initial goals (not yet achieved):
- 100% rejection of local-only restored snapshots when external witness is newer, across a frozen restoration/fault corpus.
- Exactly one witness checkpoint for 64 competing callers using one target/predecessor/decision.
- 100% idempotent recovery across all commit/acknowledgement interruption points, no double-advances and no local-only PASS.
- At least 1,000 VM restore/reset trials with all outcomes classified; hardware power loss is a separate evidence tier.
- Report p50/p95/p99 for witness CAS, local prepare/finalize, recovery and end-to-end time; also report witness availability, stale/offline time and operational recovery cost.

## Cross-layer rollback experiment (software/model tier)

The D1 SQLite integration test now snapshots the real policy database after generation-41 initialization, explicitly checkpoints the WAL, performs the signed promotion to generation 42, persists and reads back the D1 COSE boot receipt, then restores the earlier SQLite file. The restored database is internally valid and `PRAGMA integrity_check` succeeds; its target state and decision ledger are nevertheless stale. The separate in-memory witness state remains at checkpoint sequence 1/generation 42, and reconciliation must return `LocalStateRollbackDetected`.

This closes an important gap between two separate toy models: the local rollback input is now an actual copied SQLite database built by the policy-store crate, not a hand-constructed `LocalState`. It is still a test of the witness state machine in memory, **not** an independently durable remote witness or a production anti-rollback service. It does not prove resilience if both the SQLite file and witness state are restored together, or if the witness signing key/service is compromised. The new `target_state()` read API validates the persisted state before a caller builds a reconciliation input; it does not itself provide anti-rollback.

## Claim ceiling

This design does not make a software-key D1 boot report equivalent to TPM/measured boot, and does not itself prove physical power-loss safety. Any claim must name the exact external witness implementation and its trust boundary. Until a real witness is integrated, SRQ must state plainly that **whole-database rollback is not currently prevented by the local SQLite challenge ledger alone**.


## Reference state machine added to the research branch

The dependency-free Rust model lives at `tools/srq-rollback-witness-model/`; an isolated workflow tests it without changing the core SRQ qualification profile. Its scenarios cover:

- restoration of only the local database while the witness remains advanced;
- witness commit followed by a lost acknowledgement and idempotent recovery;
- prepared-local-intent before witness advance (explicitly not success);
- unavailable witness preventing a new advance;
- expiry at the witness commit boundary;
- 64 competing, distinct decisions against one checkpoint predecessor (exactly one wins);
- stale predecessor, skipped sequence, wrong target and conflicting decision reuse;
- restoring both local and witness state together, which the model intentionally shows as undetectable when there is no third independent anchor;
- mismatched witness receipt preventing local finalization; altered prepared payload with an unchanged digest-shaped field must be classified as `PreparedCheckpointConflict`, never as safe recovery. A deterministic 14-field mutation regression test requires every changed target, sequence, predecessor, digest, decision, challenge, generation, deployment, artifact, policy, corpus, or freshness field to block the safe-recovery classification, while the exact committed checkpoint remains recoverable.

This is a reference state machine, **not** the production witness. It uses digest-shaped test values and assumes the checkpoint's cryptographic evidence has already been verified; it does not compute canonical checkpoint hashes, persist an external receipt, prevent its own process/database rollback, implement network consensus, use a TPM counter, or qualify hardware/VM/power-loss behavior. The explicit test where both rollback domains are restored is important: a witness stored in the same disk image does not solve the problem.

The separate workflow is [`.github/workflows/srq-rollback-witness.yml`](../.github/workflows/srq-rollback-witness.yml). Its run is independent development evidence for this model only and is not part of the current 22-gate SRQ signed evidence bundle. The integration remains a proposal until a real witness adapter is implemented with durable idempotency, independently protected monotonic state and restart/reconciliation tests.

