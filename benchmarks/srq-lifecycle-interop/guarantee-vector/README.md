# SRQ typed assurance vector — research prototype

**Research model only.** The crate evaluates evidence already supplied by adapters. It does not cryptographically verify the evidence, contact a remote anchor, execute an effect, or prove that an adapter's promises are true. A `Verified` enum variant means a trusted upstream verifier claims to have completed the corresponding check; production wiring must make that boundary explicit. `TrustedEpoch` is an opaque capability with no public constructor; only unit tests can create synthetic tokens. Until a real verified time/epoch adapter is integrated, downstream crates cannot advance retention or compaction time, so these models are deliberately not production-usable.

## Paradigm shift

A single `PASS` hides important distinctions. This model reports independent dimensions for:

1. authorization, signature, freshness, policy, and request-binding result;
2. anti-rollback checkpoint alignment and independence;
3. dispatch-claim/request consistency;
4. effect/receipt idempotency and fencing contract; and
5. authoritative receipt binding.

It then produces a constrained recovery disposition. The overall release claim never exceeds the evidence provided by the weakest required dimension.

## Recovery decisions

- `DISPATCH_ONCE`: all required preconditions are verified and no dispatch claim exists.
- `RETRY_SAME_IDEMPOTENCY_KEY`: allowed only when the same request digest is bound to a stable key and the sink contract says the effect and durable receipt are committed atomically. If fencing is required, the sink must enforce it.
- `RESUME_TRANSACTION`: continue within the same transaction identity and domain where effect and receipt are atomic; do not create a new transaction.
- `OPERATOR_REQUIRED`: an at-most-once operation may have run but has no authoritative receipt, or a completion marker lacks the receipt. The model does not guess and does not redispatch.
- `COMPLETED_NO_REDISPATCH`: a signature-valid receipt matches the exact request, effect, and policy digests. Even an expired current authorization does not justify executing the completed operation again.
- `BLOCK`: authorization, anchor, request binding, receipt, delivery contract, or required fencing is invalid, unknown, or inconsistent.

The request digest, effect digest, and policy digest are separate: request identity includes the command intent; effect identity represents the expected applied effect; the policy digest records the authorizing policy version. A verified receipt must bind all three.

## Frozen matrix and reproducibility

The frozen Rust matrix has 26 cases: safe dispatch, same-key retry, same-transaction recovery, conservative operator escalation, verified completion, and attacks covering missing/invalid/stale authorization, request mismatch, unavailable or rolled-back anchors, divergent checkpoints, key/contract mismatch, non-atomic effects/receipts, unknown delivery guarantees, missing fencing, wrong/invalid receipts, missing completion receipts, malformed digest input, and unknown-outcome claims with missing fencing tokens or mismatched idempotency keys. An additional bounded cross-product explores 4,375 combinations across authorization, anchor, dispatch, delivery-contract, and receipt dimensions using a separate minimum-evidence oracle. The new in-memory Rust sink model adds direct tests of tenant-scoped idempotency, same-key/different-request rejection, exact-retry receipt reuse, stale fencing-token rejection, and a 64-thread same-key race.

Run from the repository root:

```sh
cargo fmt --manifest-path benchmarks/srq-lifecycle-interop/guarantee-vector/Cargo.toml --all -- --check
cargo test --locked --manifest-path benchmarks/srq-lifecycle-interop/guarantee-vector/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/srq-lifecycle-interop/guarantee-vector/Cargo.toml --all-targets -- -D warnings
cargo run --locked --manifest-path benchmarks/srq-lifecycle-interop/guarantee-vector/Cargo.toml --bin qualification-report
```

The acceptance gate is exactly 26/26 expected outcomes, zero unexpected decisions, zero false releases, and zero false blocks against the frozen expectation matrix, plus 4,375 cross-product states with zero false releases and zero false blocks under the independent minimum-evidence oracle. Cargo tests also exercise the Rust sink, retention, and epoch-compaction models; their state is process-local and not durable. CI attests `src/retention_model.rs` and `src/epoch_compaction_model.rs` alongside the evaluator and sink source. Reports remain `qualification_status=BLOCKED` and `claim_ceiling=none`.

## Reference sink model (not production)

The module `src/sink_model.rs` provides a mutex-serialized in-memory sink solely to make idempotency/fencing semantics executable in Rust. It scopes a key by tenant/operation domain; binds it to request and effect digests; returns the original receipt for an exact key replay; rejects the same key with different request/effect intent; and rejects a **new** effect from a token lower than the accepted high-water token. The 64-thread race requires one logical effect and one shared receipt.

This sink is intentionally volatile. A process crash discards it; the mutex does not establish disk durability, multi-process atomicity, distributed linearizability, trusted token issuance, or production exactly-once behavior. The sink and evaluator must not be treated as an end-to-end security proof. A production adapter needs transactional effect+receipt persistence, durable idempotency-key retention, and sink-side fencing, followed by actual crash and restore tests.

## New retention-horizon model

`src/retention_model.rs` tests a separate failure mode: idempotency is only as durable as the memory of prior keys. The sink derives retention from a policy-controlled formula:

`retain_until = authorization_expiry + maximum_retry_horizon + epoch_skew_margin`

Callers cannot request a shorter retention window. Effect submission does not accept a timestamp, and fails closed until a trusted epoch observation exists. The sink then reads a monotonic high-water epoch updated through an opaque `TrustedEpoch` capability. There is no public capability constructor yet, so downstream code cannot advance the model clock. Production must integrate a verifier that creates the token only from independently verified time/epoch evidence.

The model returns the original receipt for exact same-key retries while its retention window is open—even after authorization expiry—but never applies a new effect from that retry. Once the retention window is past, the active receipt becomes a permanent tombstone and the key cannot be used again in that namespace. This is intentionally conservative. Safely deleting tombstones requires a proof that old authorizations are irreversibly expired, the old namespace cannot be restored, and old key epochs cannot be reused. The separate abstract epoch-compaction model below tests this ordering but does not provide a production anchor.

The retention model includes 14 focused tests for derived retention horizon, replay after authorization expiry, tombstoning, changed-intent collisions, rejection of fresh expired operations, excessive authorization lifetimes, monotonic epoch regression, pre-observation fail-closed behavior, stale fencing, scope separation, 64-way concurrency, and overflow. All state is in memory. These tests do not demonstrate durable retention across process restarts, storage rollback, or an untrusted clock.

## Anchored key-epoch compaction model

`src/epoch_compaction_model.rs` tests a bounded-storage alternative to keeping every tombstone forever. Each idempotency key belongs to a monotonic namespace epoch. The old epoch can be compacted only after every receipt and tombstone in that epoch is past its retention horizon.

The model derives each key's retention deadline using the same `RetentionPolicy` formula as the retention sink—authorization expiry plus retry horizon plus epoch-skew margin. Callers cannot submit a shorter retention deadline, and `prepare_compaction` accepts no caller-supplied `now`: it reads a monotonically advanced trusted-time observation from the anchor model. A production adapter must ensure only verified time/epoch evidence can advance that observation. It enforces the order:

1. Verify that all prior-epoch retention horizons have passed and persist a local prepared transition.
2. Atomically advance an independent rollback-protected anchor from epoch `n` to `n+1`.
3. Commit the local epoch transition.
4. Only then prune receipt/tombstone records from epochs older than the new minimum.
5. Reject every request with a retired epoch before looking up or applying its effect.

This ordering preserves a small permanent high-water mark instead of retaining an unbounded key-by-key tombstone set. The model also covers crash recovery: before anchor advancement, discard the prepare and keep old keys; after advancement, finalize the matching prepared transition; after restoration of a snapshot that lacks the prepare, fail closed because the anchor is ahead.

The module has 16 tests covering policy-derived retention horizons, authorization-window bounds, overflow, trusted-time sourcing and regression, retention gating, old-epoch rejection, skipped epochs, untrusted anchors, both sides of the anchor-advance crash boundary, snapshot restoration, and pruning only after commit. It is a single-process abstract model: it does not implement durable CAS, signed anchor evidence, TPM or remote service integration, power-loss behavior, or key-epoch authorization. Production must bind the epoch to the signed request and an actually independent durable anchor before relying on compaction.

## Comparative benchmark protocol

The full matched-contract benchmark procedure is in [`BENCHMARK_PROTOCOL.md`](./BENCHMARK_PROTOCOL.md). It defines comparable reference classes, fixed workload/fault matrices, per-adapter snapshot/restore trials, safety/availability/performance metrics, and a rule against comparing unlike durability promises. Its proposed targets are not measurements or guarantees.

## Industry comparison

- AWS's idempotent API design stores the original request parameters with a client request identifier and rejects the same identifier when intent differs: https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/
- Google Cloud Pub/Sub's exactly-once delivery contract is scoped to pull subscriptions and a single region and has explicit latency and retry trade-offs: https://docs.cloud.google.com/pubsub/docs/exactly-once-delivery
- The Update Framework specifies versioned trusted metadata and defences against rollback/freeze attacks: https://theupdateframework.github.io/specification/latest/
- RFC 9334 treats freshness as a protocol architecture choice involving trusted clocks, nonces, or epoch IDs: https://www.rfc-editor.org/rfc/rfc9334.html
- NIST SP 800-193 frames platform resiliency around protection, detection, and recovery: https://csrc.nist.gov/pubs/sp/800/193/final

The benchmark target is not an unqualified claim of “better than SOTA.” It is to give a more precise, measurable contract for each lifecycle transition and demonstrate lower false release risk with a published latency/recovery cost under comparable workload and durability guarantees.

## Next qualification gates

1. Compile, format, test, and lint this crate on the exact PR head.
2. Port the types to the production Rust integration and make the “verified evidence” adapter boundary explicit.
3. Use canonical domain-separated encodings and reviewed crypto libraries for signed authorizations, receipt verification, and request/effect digests.
4. Add real anchor and effect adapters with durable idempotency/fencing semantics.
5. Inject at least 100 crashes per durable boundary and run at least 1,000 snapshot/restore/replay cases per real adapter.
6. Report false-release count, false-block count, p50/p95/p99, throughput, recovery duration, persistence cost, and operator-required rate. Do not hide availability regressions by reporting only duplicate prevention.

Until those gates have exact-head evidence, this model is an aid to design and review, not a production security claim.
