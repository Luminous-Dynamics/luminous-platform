# agent-effect-gate v0.1

Research-only Rust reference for the E0/E1 portion of Luminous Platform issue #168.

## What this prototype enforces

- Exact tool/operation/version manifest lookup; unknown, unqualified, and read-only operations cannot enter the mutation path.
- ACT is necessary but not sufficient. The grant binds issuer, scope, policy, manifest version/digest, toolchain ID/digest, principal, tenant, episode, plan, step, target, request digest, attempt, idempotency key, environment, and risk.
- Authority/manifest/policy source failures, missing preconditions, missing independent observer capability, missing participant ledger support, and missing verifiers fail closed.
- A private non-cloneable permit is consumed internally against the exact request/target/attempt before dispatch.
- Participant acceptance/effect evidence is kept separate from the gate receipt.
- The broker asks the participant-owned logical-action ledger for the latest state of the tenant-scoped idempotency key before every dispatch; it maintains no parallel attempt/effect ledger.
- An ambiguous response remains OutcomeUnknown; a distinct attempt is blocked until the participant's exact-attempt query and logical-key index consistently report matching NeverAccepted evidence.
- Fifty-five negative conformance vectors assert zero calls to the fake participant.

## Boundaries / non-claims

This is not a production broker or security certification. Grant authenticity, principal authentication, canonical request construction, digest correctness, policy-store integrity, participant-port identity, and the authority of participant evidence are adapter trust assumptions. The broker's one-use permit replay guard and receipt sequence are process-local and not crash durable; authoritative attempt and idempotency state belongs to the participant ledger. No cryptographic signing, tamper-evident append-only storage, connector isolation, multi-process replay protection, real service observation, or host transaction execution is implemented. It does not prove that a read-only tool is actually side-effect-free. Production integration must reuse Platform Saga's existing episode state and participant-owned ledgers; host operations must pass through Nixward, which remains the sole host transaction/recovery authority.

## Run

```sh
cargo test --locked
```

No external crates are required. The tests exercise only a fake participant; they do not contact production systems.


## Participant act-bound finality sink v0.1

The additive src/finality.rs module models the participant-owned Finality Sink for issue #206. The participant reconstructs a typed Candidate Act and compares it with the exact act bound into an opaque authenticated-handle wrapper. It validates the participant audience, issuer identity presence, issue/not-before/expiry interval, and current policy epoch.

Within one participant-owned critical section it consumes the one-use handle and nonce and records the protected effect plus idempotency index. The effect receipt preserves the complete reconstructed Candidate Act, authenticated handle-envelope digest, nonce digest and participant sequence; a replay result separately identifies the handle consumed for that retry. Exact handle replay returns the original receipt without a second effect. A newly authorized retry with a new attempt/grant/runtime-evidence receipt can return the original logical effect when tenant/key and stable action fingerprint match; changed plan, target, request, operation, manifest, toolchain or policy is an idempotency conflict. Competing handles for one logical action therefore produce at most one effect in this model.

Sixteen deterministic tests cover exact commit, same-handle replay, target/request/tenant/principal/plan/operation substitution, wrong audience, expiry/not-before, policy rotation, nonce replay, handle-ID reuse, retry/idempotency, changed-request conflict, malformed act/interval, and two concurrent-consumption races.

Claim ceiling: the wrapper is only structurally authenticated within this E0 reference module; there is no cryptographic signature or trust-root validation, canonical byte encoder, or trusted verifier adapter yet. The sink is in-memory and mutex-linearized, not durable or distributed. A production participant must atomically commit the business effect, idempotency record, consumed handle and nonce in one durable transaction/CAS. If its external side effect cannot share that atomic boundary, strict finality must reject or remain unsupported. This module is not wired into the existing broker dispatch path and does not establish production act-bound execution finality.
