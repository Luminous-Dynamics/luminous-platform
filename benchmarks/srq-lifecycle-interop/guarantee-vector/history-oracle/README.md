# SRQ sink-history oracle v1

## Why this layer exists

The evaluator and mutation harness answer two questions: what decision should the policy model make, and do selected regression tests notice specific unsafe code changes? Neither answers whether an actual adapter's observed operation history violates its contract. This oracle is a separate, deterministic checker for normalized histories exported by a concrete sink adapter.

The intended loop is **real adapter → fault injection → immutable operation history → independent oracle → retained report**. The oracle must not share the adapter's decision-making code. Adapter qualification remains blocked until a real adapter, independently observed effects/receipts, durability tests, and exact-head execution evidence exist.

## Run

```sh
python3 -m unittest discover -s benchmarks/srq-lifecycle-interop/guarantee-vector/history-oracle -p 'test_*.py' -v
python3 benchmarks/srq-lifecycle-interop/guarantee-vector/history-oracle/history_oracle.py benchmarks/srq-lifecycle-interop/guarantee-vector/history-oracle/fixtures/valid-retry.json
```

The CLI exits 0 for a structurally valid safety-PASS trace, 1 for a well-formed trace that violates an invariant, and 2 for invalid/uninterpretable input. A safety PASS is deliberately not a liveness PASS.

## Frozen event format

The exact event names and required fields are in `history-protocol-v1.json`. Each trace covers one `sink_identity_digest` and one explicit idempotency-key namespace/retention scope. Each request, effect, and policy is represented by a lowercase 64-hex SHA-256 digest. `seq` is a contiguous zero-based causal order, not a wall-clock timestamp. The adapter exporter must preserve enough causal ordering that receipt/effect facts referenced by a committed response appear before that response.

Event kinds:

- `invoke`: one client attempt, intent digests, key, and fencing token.
- `fence_advanced`: an observed monotonic fencing-token advance.
- `effect_committed`: an independently observed sink effect identifier and its intent/sink/fence binding.
- `receipt_committed`: an independently observed receipt identifier, effect binding, and the same intent/sink tuple.
- `response`: the attempt result (`committed`, `unknown`, or `rejected`) and optional receipt ID.

## Checked safety invariants

The checker reports stable `HIST-001` through `HIST-007` invariant IDs as well as machine-readable violation codes. The tests cross-check the ID/code mapping against the frozen protocol so a code change cannot silently drift from the research contract. It detects non-contiguous histories, duplicate attempt/effect/receipt identifiers, reuse of one idempotency key for different intent, multiple distinct effects under one key, stale or mismatched fences, effect/receipt binding mismatch, orphan effects or receipts, a `committed` response without a matching prior receipt, and a rejected attempt that nevertheless has an effect or receipt.

Invalid input is distinct from a security invariant failure. For example, a malformed digest or unsupported event kind is `INVALID_HISTORY`, not a successful negative control.

## Safety is not liveness

The report emits attempt counts, terminal/rejected/unknown responses, unresolved attempt IDs, effects, effectful intents, and receipts. It always records `liveness_observations.claim = NOT_EVALUATED`. A system that rejects every request can satisfy these safety checks yet make no useful progress; the checker therefore never interprets universal blocking as liveness success. To qualify liveness, a workload must define an allowed retry policy, deadline/availability assumptions, and a minimum useful completion rate.

## Industry baseline and differentiator

This oracle is not a new consistency algorithm and should not be compared with production services based on a synthetic trace alone. It applies established ideas to one reproducible SRQ evidence contract:

- AWS's idempotent API guidance binds a request token to original intent and rejects changed intent on token reuse: https://aws.amazon.com/builders-library/making-retries-safe-with-idempotent-APIs/.
- Jepsen-style testing records real operations under faults and checks histories instead of inferring correctness from unit-test counts: https://jepsen.io/analyses.
- RFC 9334 describes freshness as an explicit design decision and discusses epoch-ID tampering, replay, delay, and reordering: https://www.rfc-editor.org/rfc/rfc9334.html.
- TUF checks persisted metadata versions and expiry to detect rollback and freeze attacks: https://theupdateframework.github.io/specification/latest/.

The differentiator we can aim for is a versioned, adapter-neutral protocol; an independent checker; intentionally broken traces that demonstrate detection; source- and head-digested CI reports; safety and progress reported separately; and a reproducible real-sink fault campaign. There is no SOTA claim at this stage.

## Benchmark ladder

1. **Checker adequacy:** at least 12 deterministic unit tests; every named bad trace must trigger the intended violation, and valid retries must pass.
2. **Trace mutation adequacy:** mutate real histories (drop receipt, alter effect digest, replay key with changed intent, duplicate effect, regress fence) and require exact invariant IDs, not merely a nonzero exit.
3. **Adapter qualification:** derive histories from a persistent transactional sink; instrument crash points before/after effect and receipt commit, response loss, retry, concurrent same-key requests, stale workers, failover, snapshot restore, and anchor rollback.
4. **Trials:** start with 100 repetitions per durable boundary and 1,000 snapshot/restore/replay trials per adapter. Report sample sizes and confidence intervals; zero observed failures is not proof of zero risk.
5. **Comparative performance:** under equivalent durability/retention semantics, report p50/p95/p99 latency, throughput, storage/write amplification, bytes replicated, recovery time, false-block rate, false-release rate, and useful completion rate.

## Limitations

The current fixture suite and oracle are portable test tooling, not a sink adapter, durable database verifier, independent witness, cryptographic verifier, or production benchmark. The ordering semantics must be reviewed against the eventual adapter's event collection method; a dishonest or lossy recorder can conceal violations. Unknown outcomes remain unresolved. Retention expiry, compaction, and namespace migration need dedicated extensions before their histories are accepted under this protocol.
