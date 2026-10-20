# Luminous Edge challenge-bound receipt freshness v1

**Claim ceiling:** challenge and byte-binding consistency, not signer/device authentication, trusted execution, hardware attestation, or secure wall-clock proof.

## Problem

The v1 receipt reconciler binds a receipt to exact corpus bytes and checks schema, evidence source, time ordering, missing/duplicate cases, and observed outcomes. Those checks do not, by themselves, establish that an otherwise valid receipt was generated for the verifier's current request. A previously valid receipt could be presented again unless a verifier adds a freshness challenge and replay policy.

This addition is a separate verification layer. It does not change the v1 receipt schema or silently reinterpret old receipts. Callers must use both regular corpus reconciliation and this freshness layer before promoting a policy.

## Protocol

1. The verifier prepares a challenge with a cryptographically secure random 32-byte nonce, a unique challenge ID, issued/expiry timestamps, a maximum receipt-age bound, policy generation, corpus SHA-256, approved-policy SHA-256, and target token.
2. An authorized probe runner performs its normal test and produces the existing v1 receipt.
3. The runner (or a trusted wrapper) returns a freshness envelope containing the same challenge ID and nonce, run ID, policy generation, start/completion timestamps, and SHA-256 of the exact receipt bytes.
4. The verifier checks schema validity, challenge/envelope/receipt bindings, exact corpus bytes, exact receipt bytes, target and policy generation, challenge expiry, run interval, maximum receipt age, and that each observation timestamp lies within that run interval.
5. It invokes ordinary corpus reconciliation for the exact supplied corpus and receipt bytes. The freshness nonce is consumed only if the receipt is fresh **and** normal reconciliation is conformant.
6. A caller that promotes a policy must persist the consumed nonce transactionally with the promotion decision. The library's in-memory guard is only an executable contract/test helper.

## What the guard rejects

- Receipt bytes changed after the envelope was issued, even if the parsed JSON object is semantically equivalent.
- Envelope challenge ID or nonce different from the active challenge.
- Reuse of a nonce already consumed by this guard instance.
- Challenge not yet issued or already expired, challenge lifetime above one hour, or receipt age above the challenge's explicit limit (itself capped at one hour).
- Envelope run ID or policy generation different from the receipt/challenge.
- A receipt for a different corpus, approved policy digest, or target token.
- Synthetic fixtures, control-plane acknowledgements, and other non-dataplane sources in this freshness path.
- Out-of-window observation timestamps, invalid run interval, or backward monotonic trial order.
- Any receipt that fails normal frozen-corpus reconciliation.

## Schema/API

- **Challenge schema:** schemas/edge-policy-verification-challenge-v1.schema.json
- **Freshness envelope:** schemas/edge-policy-freshness-envelope-v1.schema.json
- **Implementation:** tools/edge-policy-observation-reconciler/src/freshness.rs — ChallengeReplayGuard::verify_and_consume(...)
- Existing receipt contract remains at schemas/edge-policy-observation-receipt-v1.schema.json.

The API receives verifier_now_unix_ms from the verifier side. It never treats a timestamp asserted only by the receipt as the current time.

## Security limits

- **Nonce generation is the caller's responsibility.** Use an OS cryptographic random generator, not a timestamp, PRNG seed, run ID, or counter. A schema only verifies nonce shape, not its unpredictability or source.
- **Replay state is in memory.** If the process restarts, this helper alone does not remember old nonces. Production use requires a durable challenge ledger with atomic compare-and-consume semantics bound to the promotion transaction.
- **Time is not authenticated.** An envelope and receipt can lie about timestamps unless their origin and clock are independently trusted. Challenge time checks limit accidental/stale replays; they are not a replacement for trusted time, signatures, device attestation, or authenticated channels.
- **SHA-256 is byte binding, not identity.** An attacker able to rewrite both envelope and receipt can compute new hashes. The layer does not prove who created the observation.
- **Policy generation is a caller-provided version number.** The control plane must guarantee uniqueness/monotonicity of generations per target and bind it to the actual installed policy identity.
- The normal reconciler's reported_observations_consistent_not_authenticated status is preserved explicitly in the result; successful freshness checking still has claim ceiling freshness_consistent_not_authenticated.

## Proposed benchmarks

The tests in this module exercise deterministic correctness and replay controls, not cryptographic randomness or production concurrency:

- Fresh matching challenge/receipt accepted once.
- Same nonce rejected after consumption.
- Stale receipt rejected.
- Envelope challenge ID/nonce/run ID/policy generation bound across objects.
- Exact receipt bytes and corpus bytes bound with SHA-256.
- Wrong target/policy/corpus and out-of-range observation timestamps rejected.
- Synthetic fixtures and non-dataplane evidence rejected by this layer.
- A failed attempt must not consume the nonce; a valid retry should still be accepted.

Targets for a future production implementation: zero stale/replay receipts accepted in a frozen adversarial corpus of at least 100,000 deterministic variants; zero nonce reuse in the production challenge issuer; linear-time validation in number of observations; durable concurrent nonce consumption with crash-recovery tests; and explicit false-reject and latency reporting. These are qualification targets, not achieved claims.

## Industry context

Batfish differential reachability is an established pre-deployment snapshot-analysis tool and should be used rather than recreated. Enterprise platforms already expose concepts such as policy generation, session rematching and change control. This layer's intended contribution is a portable receipt-and-challenge contract across multiple implementations, separating freshness, conformance, and authentication. It does not claim superiority to Batfish or commercial firewall systems.
