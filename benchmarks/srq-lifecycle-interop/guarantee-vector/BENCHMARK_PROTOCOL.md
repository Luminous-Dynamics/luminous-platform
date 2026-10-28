# SRQ lifecycle assurance — comparative benchmark protocol v1

**Status: proposed qualification protocol. No performance superiority result is claimed.**

This protocol is designed to compare lifecycle-assurance implementations without conflating their guarantees. It covers a privileged operation with signed authorization, anti-rollback state, a dispatch claim, a potentially non-idempotent external effect, and an authoritative receipt.

## 1. Compare guarantees before comparing speed

Every implementation must be described using the following fields before its results are compared:

| Dimension | Required declaration |
|---|---|
| Authorization | Signature algorithm/key source, policy/revocation behavior, request fields covered, freshness mechanism |
| Anti-rollback | State boundary, atomicity, durability, restore threat, failover behavior |
| Idempotency | Key namespace, request digest binding, retention horizon, behavior on key/payload mismatch |
| Fencing | Token issuer, monotonicity domain, and whether the effect sink itself rejects stale tokens |
| Receipt | Who creates it, what it binds, whether it is durable and atomic with the effect, how it is queried |
| Failure semantics | Retry, resume, operator-required, or block after each failure point |
| Durability promise | Process crash, OS crash, power loss, host restore, remote quorum/failover |
| Scope | Single process, host, region, multi-region, or cross-provider |
| Performance cost | p50/p95/p99 latency, throughput, recovery time, CPU, memory, durable writes, network round trips |

A result is only comparable with another result when the requested durability and threat models overlap. A local mutex or journal result is not comparable to a remote quorum result merely because both report “one effect.” A message-delivery guarantee is not by itself a guarantee that a downstream operating-system side effect and its receipt commit atomically.

## 2. Reference classes to compare

Run each as a separate class with its own limitations; do not collapse these into a single leaderboard.

1. **Local reference** — local journal plus process/thread serialization. Useful for measuring local overhead; does not withstand restore of a complete earlier journal without an independent anchor.
2. **Transactional sink** — a real database transaction that atomically binds the idempotency key, request/effect identity, business effect, and receipt. Report the database durability/replication mode.
3. **Independent anchor + persistent sink** — the intended SRQ production class: a real remote atomic checkpoint or properly bound hardware-backed anchor, plus a persistent effect/receipt adapter.
4. **Managed messaging control** — a managed exactly-once delivery product under its documented scope, connected to a downstream effect sink. Measure end-to-end effect duplication separately from message redelivery.
5. **Update-metadata control** — TUF-style version/rollback validation for the artifact-update layer. This is an update integrity control, not an equivalent arbitrary side-effect execution adapter.

Document every version, configuration, region, replication mode and service tier. If a provider offers an exactly-once guarantee only within a particular region or subscription type, do not extrapolate it outside that scope.

## 3. Workload matrix

Use fixed, deterministic workload definitions and publish the raw per-run observations.

### Request volume and contention

- Worker counts: 1, 8, 64, 256, and 1,024 where the adapter can sustain them.
- Distinct logical operation counts: 1, 100, 10,000, and 1,000,000 for retention/storage tests.
- Same-key concurrency: 1, 8, 64, 256 simultaneous submissions to one key.
- Independent-key traffic: use random distinct scoped keys and a separate skewed distribution where a small fraction of hot keys receive most requests.
- Retry pattern: first request, immediate retry, retry after response loss, retry just before authorization expiry, retry exactly at retention horizon, retry strictly after retention horizon.
- Collision controls: same key/same intent; same key/different request digest; same key/different effect digest; same key/different policy; same token in another tenant/operation scope.
- Fencing controls: stale token after new owner commit; same-key receipt lookup after fence advance; missing token; token reused across scopes.

### Fault injection

Inject an abrupt process kill and record recovery behavior at each boundary:

1. before prepared record;
2. after prepared record write but before sync;
3. after prepared record sync;
4. before anchor compare-and-advance;
5. after anchor commit but before local commit;
6. after local commit but before dispatch;
7. after sink effect but before caller receives response;
8. after effect reply but before local completion record;
9. after receipt/tombstone update;
10. during key-epoch rotation and pruning.

The initial engineering gate is at least **100 trials per failure boundary**. Before production qualification, run at least **1,000 snapshot/restore/replay trials per real adapter**, including restore of the local journal alone, old VM snapshot, anchor outage, anchor failover, and device reset/reprovisioning scenarios. More trials may still be required for the target confidence level; zero observed failures in finite trials is not proof of zero failure probability.

### Retention and epoch-compaction controls

- Derive retention deadline from authorization expiry + retry horizon + skew/propagation margin; never accept a shorter caller-provided deadline.
- Test exactly-at-boundary and strictly-after-boundary behavior.
- Move trusted time backwards; expect rejection without changing the high-water mark.
- Try to invoke the effect using an old namespace epoch after anchor advance; expect rejection before lookup or dispatch.
- Crash before anchor advance; the unanchored prepare is discarded, and no old records are pruned.
- Crash after anchor advance; only the matching prepare may be finalized.
- Restore a local snapshot that lacks the prepared transition after anchor advance; recovery must block.
- Exercise tombstone compaction only after all records in the retired namespace are beyond retention.

## 4. Report these outcomes

Publish one machine-readable report per exact source revision, with source digests and adapter configuration digest.

### Safety

- false authorization accepts;
- false request/effect/policy binding accepts;
- false rollback accepts;
- duplicate logical effects;
- wrong or unverifiable receipts accepted;
- stale-fencing-token accepts;
- old-epoch accepts after anchor rotation;
- malformed or partial state silently accepted.

**Required safety target: zero across the frozen controls and no unsafe dispatch in the declared tested crash/restore schedule matrix.** Report any failure with the exact minimal reproducer; never average it away.

### Availability and recovery

- valid requests wrongly blocked;
- operator-required rate;
- successful completions per clean run;
- recovery decision after each fault point;
- time from restart to safe disposition;
- fraction of operations that require a fresh authorization after an ambiguous outcome.

**Proposed clean-state target:** at least 99.9% of valid requests accepted under the declared no-fault workload, with zero false releases. This is a test target, not a claim about current code or any vendor system. Keep the clean-state denominator and faulted-state denominator separate.

### Performance and resources

- end-to-end p50/p95/p99 and maximum observed latency;
- attempts/sec and completed logical effects/sec;
- anchor round trips, database transactions, durable syncs and remote writes per operation;
- CPU, peak RSS, storage growth per million keys, and recovery time;
- clean-state overhead relative to a baseline with the **same** durability promise;
- tail latency and availability during anchor partition/failover.

An initial optimization target worth evaluating—not assuming achieved—is no more than **20% p99 latency overhead** versus the best baseline that provides the same authorization, anti-rollback, effect+receipt atomicity, retention and durability guarantees. If the requirement cannot be met, report which guarantee causes the cost and present a Pareto curve, rather than weaken the security gate.

## 5. Statistical interpretation

Report per-run distributions and confidence intervals, not only pooled averages. A rough “rule of three” gives an upper 95% bound near 3/n for a failure probability when zero failures appear in n independent trials. Thus, zero failures in 100 trials is consistent with a several-percent rate; zero in 1,000 is still only evidence bounded by the trial model, not proof of impossibility. Crash trials may be correlated, so vary schedules, machine state, timing, snapshots and adapter failure modes.

Do not report only “duplicate effects = 0.” Pair it with false-block rate, operator-required rate, recovery time and storage growth. A system that blocks all work can trivially avoid duplicates and still be unusable.

## 6. Acceptance gates

| Gate | Pass criteria |
|---|---|
| G0 — Model integrity | Frozen matrix passes; the independent minimum-evidence oracle reports zero false releases and false blocks across the published bounded state cross-product |
| G1 — Rust implementation | Exact-head rustfmt, locked tests, Clippy with warnings denied, source-digested report |
| G2 — Persistent sink | Atomic effect+receipt transaction, scoped idempotency, mismatch rejection, durable receipt, sink-enforced fencing |
| G3 — Independent anchor | Authenticated monotonic state and atomic durable compare-and-advance outside the local restore boundary |
| G4 — Failure resilience | 100 crashes per boundary plus 1,000+ snapshot/restore trials per real adapter; zero unsafe dispatch/duplicate effect in supported contracts |
| G5 — Operational comparison | Same workload and durability mode; p50/p95/p99, throughput, resource costs, recovery and false-block rates published |
| G6 — Production claim | Independent review, explicit assumptions/trust boundaries, rollback/rotation recovery procedure, signed source/configuration evidence |

Current PR models only target G0 and parts of G1. Their outputs must remain `qualification_status=BLOCKED` and `claim_ceiling=none` until real adapters and later gates pass.

## 7. Industry references and scope notes

- AWS Builders' Library, *Making retries safe with idempotent APIs*: same client token with changed request intent should be rejected; retaining the original request parameters is essential. https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/
- Google Cloud Pub/Sub, *Exactly-once delivery*: the contract is pull-subscription and region-scoped, has additional coordination/latency, and does not make arbitrary downstream side effects atomic. https://docs.cloud.google.com/pubsub/docs/exactly-once-delivery
- TUF specification v1.0.37: versioned trusted metadata and rollback/freeze defences for software update systems; not a general operating-system side-effect transaction. https://theupdateframework.github.io/specification/latest/
- RFC 9334, section 10: freshness depends on explicit trusted time, verifier-issued nonces, or epochs, with protocol/latency/race trade-offs. https://www.rfc-editor.org/rfc/rfc9334.html
- NIST SP 800-193: platform resilience organized around protection, detection and recovery. https://csrc.nist.gov/pubs/sp/800/193/final

The differentiator to aim for is not an unqualified “exactly once.” It is a typed, replay-safe lifecycle contract with explicit proof boundaries, bounded retention, safe epoch compaction, measured recovery cost, and independently reproducible failure evidence.
