# SRQ anti-rollback checkpoint model

**Research model only.** This crate is not a production anti-rollback implementation, an authorization verifier, or a release qualification. It intentionally has no third-party dependencies and does not implement hashes, signatures, storage, TPM calls, remote services, or privileged execution.

## Paradigm shift: anchor state outside the thing being restored

The durable local challenge ledger serializes consumption and syncs journal records, but a complete older copy of that journal can erase evidence that a challenge was consumed. A hash chain stored only in the same rollbackable filesystem does not fix this: restoring the old chain restores its old valid head too.

The new invariant is **the authoritative checkpoint must live outside the attacker's journal-snapshot boundary**. The local journal is a recoverable cache; an independent anchor is the anti-rollback authority. Reconciliation compares both sequence and root. Any impossible relationship blocks rather than choosing whichever copy appears newer.

The trait in `src/lib.rs` describes the minimum adapter contract: read a checkpoint and atomically compare-and-advance from an exact expected checkpoint to its immediate successor. `MemoryAnchor` only demonstrates stale-writer rejection in tests; it is not a security boundary.

## Recovery protocol to review

A checkpoint is a pair `(sequence, root)`. In a production encoding, `root` must be a cryptographic digest over a domain-separated, canonical encoding of the previous root, sequence, transaction/challenge identity, authorization binding, and event. This model treats 32-byte roots as opaque fixture values and does not implement that encoding.

1. Verify the signed authorization, signer/policy/revocation state, time/freshness policy, challenge ID entropy, and every transaction/target/plan/artifact/policy binding. The existing ledger currently requires its caller to do this.
2. Construct the next event and prepare record. Write the prepared record and durably sync it before touching the anchor.
3. Atomically advance the independent anchor from `(n, root_n)` to `(n+1, root_(n+1)` only if the expected old checkpoint still matches. The adapter must document what "durable" and atomic mean under power loss, concurrency, device reset, and restore.
4. Mark the journal checkpoint committed and durably sync it.
5. Only after the anchor transition and journal commit succeed may the caller begin a privileged side effect.

### Recovery decisions

- Local committed checkpoint equals anchor, with no prepare: continue.
- Local committed checkpoint equals anchor, with a well-formed unanchored prepare: discard that prepare and require fresh authorization; no side effect was permitted yet.
- Anchor is exactly one checkpoint ahead and a matching prepared record is present: finalize the anchored checkpoint. The action might never have run, so retain at-most-once safety even if it sacrifices liveness.
- Older local journal, a missing prepare for an advanced anchor, divergent root, sequence gap, invalid prepare, unavailable anchor, or ambiguous state: fail closed and require explicit recovery. Never infer permission from a locally restored snapshot.

If the process crashes after anchoring but before the journal commit, recovery may mark the challenge spent even though no side effect ran. This is intentional. Exactly-once side effects across a non-transactional OS action require a separate idempotency/transaction contract; this checkpoint protocol alone does not supply exactly-once execution.

## Anchor choices and trust boundaries

A production adapter must select and justify one of the following rather than claim generic "hardware-backed" security:

- A remote, authenticated service with an atomic compare-and-swap checkpoint, durable quorum semantics, access control, and explicit behavior during partition/failover.
- A hardware-backed monotonic counter **plus** a design binding the checkpoint root to protected state. A counter by itself does not authenticate an arbitrary local journal or atomically update a root in a second storage object.
- A multi-witness transparency design that signs checkpoints and checks consistency against previously observed heads. This can detect rollback/equivocation only if clients retain or exchange trusted prior checkpoints; one isolated log is not enough.

The trust model must say what happens if the remote anchor is compromised, TPM/NV is cleared or reprovisioned, device replacement occurs, a tenant restores both VM and anchor, keys are rotated, or network access disappears. Recovery must not silently lower the high-water checkpoint. A fresh enrollment or operator-mediated recovery should be an explicit distinct policy, not an automatic "reset" path.

## Threats covered by the abstract model

The frozen matrix in `src/lib.rs` contains 12 cases: three positive controls and nine hostile or impossible states, including restoration of a previous complete journal, multi-checkpoint rollback, a journal ahead of the anchor, same-sequence root fork, invalid prepared previous root, sequence skip/overflow, and anchor/prepared-root divergence. The benchmark prints expected/observed counts and each case result. This tests state-machine behavior, not cryptographic strength or real persistence.

## Next qualification gates

1. **State machine:** 12/12 fixtures match; zero false accepts, false rejects, or unexpected outcomes.
2. **Property-based exploration:** enumerate bounded operation/crash schedules around prepare write, journal sync, anchor CAS, commit write, commit sync, and pre-side-effect boundary; no trace permits a second consume after the anchor advances.
3. **Fault injection:** 100 kills per durable boundary; at least 1,000 storage/VM snapshot restore and replay trials per adapter. Missing or ambiguous state must block.
4. **Anchor semantics:** demonstrate linearizability under concurrent writers, durable CAS under reboot/failover, and that the attacker cannot restore both journal and anchor as one snapshot.
5. **Operational benchmark:** report p50/p95/p99 and throughput for local model, remote quorum anchor, and selected hardware adapter under nominal load, 10x contention, network partition, cold restart and recovery. Publish workload, hardware, geographic RTT, durability mode and failures; do not compare incomparable latency numbers.
6. **End-to-end:** independently verify signed authorization, keys/revocation, trusted freshness, actual NixOS preflight, side-effect idempotency, and restore/rollback behavior. Until these gates have evidence, keep `qualification_status=BLOCKED` and `claim_ceiling=none`.

## Industry comparison and intended differentiator

The design borrows established ideas rather than claiming novelty for monotonic state: TUF persists trusted metadata versions and detects rollback/freeze; RFC 9334 treats freshness as an architectural decision with clock, nonce, and epoch trade-offs; RFC 9162 uses Merkle consistency proofs and auditing to detect non-append-only or inconsistent log views. This SRQ model specializes the problem to **single-use privileged lifecycle transitions** and crash recovery between local journal durability and external checkpoint advancement.

The measurable goal is not "faster than the industry" yet. It is a falsifiable contract: every old snapshot after a successful anchor advance is rejected; every inconsistent state blocks; recovery never re-authorizes a consumed challenge; and cost/latency is reported for a concrete anchor adapter.

## References

- RFC 9334, Remote ATtestation procedureS (RATS) Architecture, especially freshness: https://www.rfc-editor.org/rfc/rfc9334.html
- The Update Framework specification, rollback and freeze attack handling: https://theupdateframework.github.io/specification/latest/
- RFC 9162, Certificate Transparency Version 2.0, Merkle consistency proofs and auditing: https://www.rfc-editor.org/rfc/rfc9162.html
- Trusted Computing Group, TPM 2.0 Library specification: https://trustedcomputinggroup.org/resource/tpm-library-specification/

**Qualification statement:** Passing the model proves only that the abstract reconciliation function matches this frozen fixture matrix. It does not prove that a real remote service or TPM is atomic, durable, authenticated, non-rollbackable, or correctly deployed.
