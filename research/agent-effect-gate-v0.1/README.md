# agent-effect-gate v0.1

Research-only Rust reference for the E0/E1 portion of Luminous Platform issue #168.

## What this prototype enforces

- Exact tool/operation/version manifest lookup; unknown, unqualified, and read-only operations cannot enter the mutation path.
- ACT is necessary but not sufficient. Current grant, issuer, scope, policy version, principal, tenant, episode, plan, step, target, request digest, attempt, idempotency key, environment, and risk must all match.
- Authority/manifest/policy source failures, missing preconditions, missing independent observer capability, missing participant ledger support, and missing verifiers fail closed.
- A private non-cloneable permit is consumed internally against the exact request/target/attempt before dispatch.
- Participant acceptance/effect evidence is kept separate from the gate receipt.
- The broker asks the participant-owned logical-action ledger for the latest state of the tenant-scoped idempotency key before every dispatch; it maintains no parallel attempt/effect ledger.
- An ambiguous response remains OutcomeUnknown; a distinct attempt is blocked until the participant's exact-attempt query and logical-key index consistently report matching NeverAccepted evidence.
- Fifty-one negative conformance vectors assert zero calls to the fake participant.

## Boundaries / non-claims

This is not a production broker or security certification. Grant authenticity, principal authentication, canonical request construction, digest correctness, policy-store integrity, participant-port identity, and the authority of participant evidence are adapter trust assumptions. The broker's one-use permit replay guard and receipt sequence are process-local and not crash durable; authoritative attempt and idempotency state belongs to the participant ledger. No cryptographic signing, tamper-evident append-only storage, connector isolation, multi-process replay protection, real service observation, or host transaction execution is implemented. It does not prove that a read-only tool is actually side-effect-free. Production integration must reuse Platform Saga's existing episode state and participant-owned ledgers; host operations must pass through Nixward, which remains the sole host transaction/recovery authority.

## Run

```sh
cargo test --locked
```

No external crates are required. The tests exercise only a fake participant; they do not contact production systems.
