# agent-effect-gate v0.1

Research-only Rust reference for the E0/E1 portion of Luminous Platform issue #168.

## What this prototype enforces

- Exact tool/operation/version manifest lookup; unknown, unqualified, and read-only operations cannot enter the mutation path.
- ACT is necessary but not sufficient. The grant binds issuer, scope, policy, manifest version/digest, toolchain ID/digest, principal, tenant, episode, plan, step, target, request digest, attempt, idempotency key, environment, and risk.
- Authority, manifest, policy, preflight and observer evidence are separate signed roles with exact proposal binding, pinned issuer/subject/key/trust-domain policy, schema versions, freshness windows, expiry and source sequence.
- The verified wrapper obtains the authority grant from the verified Authority-role statement; it does not accept a separate caller-supplied grant.
- The observer must use a control domain independent of the executor and every other evidence publisher.
- A trusted clock and a durable, atomic evidence-sequence replay guard are required by the wrapper interfaces. The in-memory replay guard is test-only.
- Cryptographic verification is deliberately an adapter contract. A production adapter must verify DSSE/in-toto envelopes and publisher identities using a maintained Sigstore/Cosign or SPIFFE integration; this crate does not implement signatures or claim an E2 qualification.
- A private non-cloneable permit is consumed internally against the exact request/target/attempt before dispatch.
- Participant acceptance/effect evidence is kept separate from the gate receipt.
- The broker asks the participant-owned logical-action ledger for the latest state of the tenant-scoped idempotency key before every dispatch; it maintains no parallel attempt/effect ledger.
- An ambiguous response remains OutcomeUnknown; a distinct attempt is blocked until the participant's exact-attempt query and logical-key index consistently report matching NeverAccepted evidence.
- Fifty-five negative conformance vectors assert zero calls to the fake participant.

## Boundaries / non-claims

This is not a production broker or security certification. Grant authenticity, principal authentication, canonical request construction, digest correctness, policy-store integrity, participant-port identity, and the authority of participant evidence are adapter trust assumptions. The broker's one-use permit replay guard and receipt sequence are process-local and not crash durable; authoritative attempt and idempotency state belongs to the participant ledger. Evidence sequence anti-replay is delegated to an external durable replay-guard implementation; the shipped fake is only a test fixture. No production cryptographic verifier, key lifecycle, durable trust-policy/revocation source, append-only receipt transparency log, connector isolation, production replay guard, real service observation, or host transaction execution is implemented. It does not prove that a read-only tool is actually side-effect-free. Production integration must reuse Platform Saga's existing episode state and participant-owned ledgers; host operations must pass through Nixward, which remains the sole host transaction/recovery authority.

## Run

```sh
cargo test --locked
```

No external crates are required. The tests exercise only a fake participant; they do not contact production systems.
