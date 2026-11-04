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

Within one participant-owned critical section it consumes the one-use handle and nonce and records the protected effect plus idempotency index. The effect receipt preserves the complete reconstructed Candidate Act, authenticated handle-envelope digest, nonce digest and participant sequence; a replay result separately identifies the handle consumed for that retry. Exact handle replay returns the original receipt without a second effect. A newly authorized retry with a new attempt/grant/runtime-evidence receipt can return the original logical effect when tenant/key and stable effect fingerprint match. The exact retry still needs a current handle, but policy version/digest/epoch, authorization scope, risk classification, and runtime evidence are not part of the business-effect fingerprint; policy rotation cannot turn an already committed operation into a second effect. Changed plan, target, request, operation, manifest, toolchain or environment is an idempotency conflict. Competing handles for one logical action therefore produce at most one effect in this model.

Twenty deterministic tests cover exact commit, same-handle replay, target/request/all 25 Candidate Act field substitutions (including participant identity and audience), wrong audience, expiry/not-before, policy rotation, nonce replay, handle-ID reuse, retry/idempotency, changed-request conflict, malformed act/interval, and two concurrent-consumption races.

Claim ceiling: the wrapper is only structurally authenticated within this E0 reference module; there is no cryptographic signature or trust-root validation, canonical byte encoder, or trusted verifier adapter yet. The sink is in-memory and mutex-linearized, not durable or distributed. A production participant must atomically commit the business effect, idempotency record, consumed handle and nonce in one durable transaction/CAS. If its external side effect cannot share that atomic boundary, strict finality must reject or remain unsupported. This module is not wired into the existing broker dispatch path and does not establish production act-bound execution finality.


The effect fingerprint is deliberately separate from the authorization envelope: the handle binds every Candidate Act field and is revalidated on each submission, while idempotency binds the stable effect semantics. This avoids duplicate commits after policy rotation without weakening act-specific authorization.


## Cryptographic integration profile v0.2

The candidate stack is deliberately compositional and remains **unimplemented**:
- `jsonwebtoken = 11.1.0` for one-use JWS/JWT act authorization.
- `vr-jcs = 0.4.1` for strict raw JSON admission and RFC 8785 canonicalization. The preferred digest candidate is domain-separated BLAKE3 (`org.luminous-dynamics.act-finality.v1`). The crate's SHA-256 strategy is declared but currently returns `UnsupportedAlgorithm`; do not use it.
- `spiffe = 0.19.0` is a candidate Workload API/SVID library. Its changelog lists 0.19.0, while the docs.rs top-level API page currently renders 0.18.0; confirm the exact 0.19.0 package and feature set in the spike lockfile before adoption.
- Sigstore Rust `0.15.1` for signed build/tool/manifest provenance, never as act authority.
- EAT only when it fits the existing #196 attestation profile; DSSE for supply-chain envelopes through maintained Sigstore tooling, not the archived standalone Rust DSSE repo.

### Non-negotiable verifier settings

For `jsonwebtoken`, override all defaults: permit one asymmetric algorithm only (initial candidate EdDSA), pin each `kid` to one issuer and algorithm in a local verifier-owned registry, set exact issuer and participant audience, require `exp`, `nbf`, `aud`, `iss`, enable `validate_nbf`, set explicitly bounded leeway (not the library's 60-second default), and manually validate `iat`, `jti`, nonce, policy epoch, act digest and token lifetime. Reject unapproved header fields and never fetch `jku`/`x5u`/keys from untrusted headers. Strict-parse the raw header and payload before normal claim deserialization to reject duplicate JSON keys.

For `vr-jcs`, use the strict byte-slice parsing path for untrusted JSON; reject duplicate names, invalid I-JSON strings/numbers and nesting >128. Serialize epochs/sequences/timestamps that can exceed IEEE-754 exact integer range as decimal strings. Recompute the act digest at the participant from the exact request it will execute. The selected digest algorithm and domain context must be versioned in the receipt.

### Verification references checked 2026-11-04

- [jsonwebtoken 11.1.0](https://docs.rs/jsonwebtoken/11.1.0/jsonwebtoken/struct.Validation.html): default required claim set is only `exp`; default leeway is 60s; `nbf` validation defaults off; algorithms default to HS256. Override all of these.
- [vr-jcs 0.4.1 API](https://docs.rs/vr-jcs/0.4.1/vr_jcs/): strict raw-byte parser, duplicate-key rejection, I-JSON checks, max depth 128, domain-separated BLAKE3 strategy; SHA-256 strategy currently unsupported.
- [SPIFFE X.509-SVID specification](https://spiffe.io/docs/latest/spiffe-specs/x509-svid/): use trust-domain/identity checks and standard certificate path validation at the actual peer boundary.
- [Sigstore Rust release 0.15.1](https://github.com/sigstore/sigstore-rust/releases): candidate verifier release; the live conformance dashboard reports its own tested commit separately from the tag, so do not overstate the 140/140 result as proof of the tag or product integration.
- [EAT RFC 9711](https://www.rfc-editor.org/rfc/rfc9711): attested claims framework, not an application-specific act authorization protocol.

This record is a candidate integration profile and known-answer plan, not a cryptographic implementation or production security claim. No new dependencies have been added to `agent-effect-gate`; do not wire crypto into production dispatch until an exact lockfile, issuer policy, signature vectors, JCS vectors and participant crash-cut tests pass.
