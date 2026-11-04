# Act-bound execution finality: cryptographic integration profile v0.1

Status: research decision record; candidate profile only. No cryptographic verifier is wired into the reference sink by this document.

Date: 2026-11-04

## 1. Objective and trust separation

Issue #206 requires the participant effect boundary to establish that this exact operation, for this exact tenant/principal/plan/target/attempt, at this exact participant, was authorized once and that the authority was consumed atomically with the effect.

Do not collapse these separate assertions into one verified=true flag:

| Assertion | Mechanism | What it establishes | What it does not establish |
|---|---|---|---|
| Build/tool/artifact provenance | Sigstore bundle verification against pinned trust root and identity policy | Which CI/workflow identity signed exact artifact bytes; transparency/timestamp evidence under the selected profile | That an agent may execute an operation or that the service effect is correct |
| Runtime workload identity | SPIFFE SVID and trust bundle, preferably mTLS | Which workload is communicating with the participant/broker | Permission to mutate a target or correctness of the workload's statements |
| Runtime attestation | A verified attestation result (EAT/RATS profile or existing #196 adapter) | Claims about a measured workload/platform under a declared appraisal policy | Operation authorization, business intent, or service postcondition |
| Act-specific authorization | One-use, audience-bound JWS/JWT execution handle with exact Candidate Act claims | A trusted issuer authorized a specific act, audience and validity interval | That the operation actually committed |
| Effect finality | Participant-owned atomic handle/nonce consumption plus idempotency/effect commit | The protected effect and consumption of authority share one participant transaction boundary | Production durability until implemented and tested in the real participant store |
| Service truth | Independent observer under qualified evidence profile | Whether the declared service postconditions and workload SLOs were met | The agent's internal reasoning or tool narration |

## 2. Candidate wire profile

### 2.1 JWS/JWT for the act-bound handle

Use a signed JWT/JWS as the initial candidate wire envelope, based on RFC 7515 / RFC 7519 and JWT Best Current Practice RFC 8725. Candidate Rust implementation: jsonwebtoken 11.1.0, pinned exactly during the integration spike.

The v0.1 handle profile should require:
- explicit typ for this profile (for example application/luminous-act-finality+jwt);
- explicit asymmetric algorithm allowlist (EdDSA or ES256 after key/HSM compatibility testing); never select the accepted algorithm from untrusted token input;
- one key ID (kid) resolved from a verifier-owned issuer/key registry; each key is pinned to exactly one algorithm;
- exact issuer, participant audience, token type and profile version;
- iat, nbf, exp, unique jti, and nonce digest;
- bounded lifetime and explicit clock-skew allowance;
- exact act digest plus policy/grant/manifest/toolchain/runtime-evidence references;
- rejection of unknown critical profile versions, wrong typ, missing required claims, duplicate JSON object names, and unsupported/ambiguous algorithms;
- issuer key rotation with explicit overlap/retirement policy and revocation/epoch invalidation.

Do not accept a valid signature from an unapproved issuer. Do not accept alg=none, symmetric/asymmetric algorithm confusion, a token intended for another participant, or a token meant for another token class.

### 2.2 Canonical Candidate Act

Define a versioned canonicalization profile over the typed Rust Candidate Act. The exact participant-side reconstructed request is the input; never trust an agent-supplied digest as proof of request bytes.

RFC 8785 JSON Canonicalization Scheme (JCS) is a candidate serialization because it defines deterministic JSON property ordering and primitive serialization. RFC 8785 is Informational, not Internet Standards Track; the project must treat it as an explicitly pinned interoperability profile, not claim standards-track status.

Constraints:
- Reject duplicate property names before canonicalization. A last-key-wins parser is not acceptable for signed security input.
- Use the I-JSON subset. Encode all 64-bit epochs, sequences, timestamps and other integers which can exceed the exact IEEE-754 integer range as decimal strings in the signed schema; do not serialize arbitrary u64 as JSON number.
- Specify Unicode normalization policy (do not silently normalize after signing); preserve exact validated UTF-8 values.
- Use a fixed, versioned field schema and reject unknown fields for the v1 action binding unless a later profile explicitly defines an extension mechanism.
- Recompute canonical bytes and digest at the participant, from the exact structured request to execute.

The integration spike should evaluate serde_json_canonicalizer 0.4 and jcs-canonicalize side-by-side against official JCS vectors and the Platform action corpus. Select one implementation only after differential tests, duplicate-key handling, large-integer behavior and fuzzing are reviewed. Do not write a custom canonicalizer.

### 2.3 SPIFFE for workload identity, not act authority

Candidate runtime identity: spiffe Rust crate 0.19.0 (checked 2026-11-04) with only the required opt-in feature(s), and SPIRE or another qualified SPIFFE Workload API provider.

Use X.509-SVID for broker/participant mTLS identity where practical. Validate:
- trust-domain and SPIFFE ID against the verifier-owned workload registry;
- certificate path and current trust bundle;
- exactly one URI SAN carrying exactly one SPIFFE ID (reject multiple URI SANs);
- validity, rotation and trust-bundle updates;
- peer identity at the actual effect boundary, not only at a proxy or upstream gateway.

JWT-SVID may be used for workload identity assertions when an X.509 channel is not suitable, but it is not the same JWT profile as the act-bound execution handle. Use distinct typ, issuer, audience, keys/trust policy and validation rules to prevent cross-JWT confusion.

A SPIFFE ID is authenticated workload identity, not authorization. Do not issue mutation authority merely because the caller has an expected SPIFFE ID.

### 2.4 Sigstore for tool/evaluator provenance

Candidate supply-chain verifier: the official sigstore-rust workspace release 0.15.1, using narrowly selected crates (sigstore-verify, sigstore-trust-root, sigstore-types) and TUF-updated trust root under a pinned update policy.

Use it for runner, evaluator, tool binary, connector image, manifest and policy-bundle artifact provenance; exact signer identity (workflow/repository/ref) and issuer policy; and bundle signature, certificate chain, transparency inclusion and timestamp verification as required by the selected Sigstore bundle profile.

Do not use a Sigstore artifact signature as a one-use operation permit. A build signature authenticates artifact provenance; it does not authorize a particular tenant/target/action.

The current Sigstore Rust 0.15.1 docs show a verification API with explicit signer identity and OIDC issuer policy. Its upstream conformance report last updated 2026-11-02 shows 140/140 tests passing for sigstore-rust @ e441356, including Rekor v2, DSSE-as-Hashedrekord and user-managed keys. This is interoperability evidence for the declared conformance surface, not evidence of production integration or broad security certification.

### 2.5 DSSE and EAT boundaries

- DSSE is appropriate for signed in-toto/Sigstore provenance and arbitrary attestation payload envelopes. The standalone probityai-archive/dsse Rust repository is archived (confirmed through GitHub repository metadata on 2026-11-04); do not adopt it as a production dependency without forking, security review, ownership, releases and conformance maintenance. Prefer Sigstore Rust's supported bundle/verifier path for provenance.
- RFC 9711 EAT is an Internet Standards Track attestation token framework based on CWT/JWT. It can carry measured-state claims, nonce/freshness and attester context under a defined EAT profile. EAT does not define an application-specific mutation authorization flow or message flow. Use EAT only through the existing #196 attestation adapter if the claims/profile actually fit; do not create a second general attestation system.
- Mycelix QUAL-001E / ASSURE-PORTFOLIO-CUT and LEX-NET-030 remain the owners for assurance-role and evidence-dependency correlation. This profile must consume those outputs rather than introduce another independence score or trust graph.

## 3. Atomic participant finality contract

At the participant boundary:
1. Authenticate the caller's SPIFFE identity over the actual channel.
2. Reconstruct the exact Candidate Act from the concrete operation and current participant request.
3. Verify the act handle's JWS/JWT signature with an allowlisted asymmetric algorithm, approved issuer/key ID, exact typ, exact participant audience, exact policy epoch and bounded validity.
4. Recompute canonical act bytes/digest and compare with the signed claim.
5. Check current grant/policy/revocation state and action scope at finality time.
6. In a single durable serializable transaction or equivalent linearizable conditional write, consume jti and nonce, check tenant/logical-action idempotency, record the protected business effect, and write the participant receipt binding the exact act/handle digest.
7. If an identical logical effect is already committed, consume the new valid handle and return the original effect receipt without repeating the effect.
8. If any precondition is stale, mismatched, unknown, revoked, expired, or unverifiable, do not commit; return a typed denial or OutcomeUnknown only when submission/commit status is genuinely ambiguous.

If the side effect occurs in an external system that cannot share the transaction boundary, the adapter must use that external system's own idempotency key and authoritative attempt/effect reconciliation. A local database transaction that merely writes 'about to call external API' does not prove atomic finality. Unsupported external effects remain denied in strict-finality mode until a participant-specific design closes the gap.

## 4. Rust integration spike — no production wiring yet

Before selecting the final dependency stack:
1. Pin jsonwebtoken 11.1.0, spiffe 0.19.0, Sigstore Rust 0.15.1, and one JCS implementation in an isolated spike branch.
2. Build a profile registry for issuer, key ID, algorithm, participant audience, token type and schema version. Do not permit token headers to fetch arbitrary URLs or trigger untrusted key discovery.
3. Build exact JWS vectors for valid handle, wrong issuer, unknown kid, wrong algorithm, HS/RS confusion, wrong typ, wrong audience, wrong participant, expired/not-yet-valid, policy epoch rotation, revoked issuer key, replayed jti, duplicate JSON keys, altered payload, changed request/target, large integer boundaries and unknown critical fields.
4. Use official JCS vectors plus cross-language Rust/Go/TypeScript vectors; fuzz duplicate keys, Unicode, numbers, parser ambiguity, truncation and nested payload limits.
5. Validate workload identity independently: valid/expired/revoked SVID, wrong trust domain, multiple URI SANs, stale trust bundle, rotation during request, and peer identity substitution.
6. Verify signed tool/manifest images with Sigstore official conformance fixtures, including malicious but correctly signed artifact from a disallowed workflow identity.
7. Use Mycelix's source-dependency/cut-set profile to ensure signer, issuer, builder, runtime, participant and observer independence claims are not inferred from distinct key IDs alone.
8. Benchmark verify latency, p50/p95/p99, key-cache hit/miss, trust-bundle refresh, throughput, contention on nonce consumption, effect ledger transaction duration, memory, failure modes and cost.
9. Test multi-process and crash-cut behavior on a durable participant adapter before describing the handle as crash-safe.

## 5. Security acceptance gates (hypotheses, not results)

- 0 successful commits across every invalid, expired, revoked, mismatched, wrong-audience, wrong-policy, wrong-target, wrong-request, wrong-manifest, wrong-toolchain, replay or duplicate-key negative vector.
- 100% participant requests reconstructed and canonicalized independently at the effect boundary.
- 100% successful effects have one exact act/handle/nonce/participant receipt chain; zero committed effects without consumed handle and nonce.
- 100% duplicate logical-effect retries return the original effect receipt; zero duplicate external effects, including after policy rotation and process restart.
- 100% missing/unknown/indeterminate identity, attestation or ledger state fails closed or remains OutcomeUnknown; never silently authorizes.
- p95 in-process finality verification overhead target <=10 ms with cached keys/trust, measured separately from remote key/trust retrieval and durable commit.
- p95 total participant commit overhead target <=25 ms only as a synthetic/disposable benchmark goal; measure on an explicitly pinned database/runtime/hardware.
- False denial <=1% on a validated positive-control corpus; report confidence interval and exact stratum.
- No claim of production security, FIPS compliance, distributed atomicity or standard compliance follows from unit/conformance tests alone.

## 6. Decision summary

Candidate technology choices:
- JWS/JWT (jsonwebtoken 11.1.0) for one-use act-bound authorization, subject to security-vector audit and explicit algorithm/key policy.
- JCS as a pinned canonicalization profile with strict I-JSON restrictions and large integers encoded as strings; test serde_json_canonicalizer 0.4 against the official corpus.
- SPIFFE/SPIRE (spiffe 0.19.0) for runtime workload identity and mTLS, never as mutation authority.
- Sigstore Rust (0.15.1) for build/tool/manifest provenance, never as an action permit.
- EAT RFC 9711 only where an existing runtime attestation profile needs a standard attested-claims envelope.
- DSSE for supply-chain provenance, but no direct adoption of the archived standalone Rust DSSE crate.

All versions above are candidate pins checked on 2026-11-04, not yet dependencies of the agent-effect-gate crate. The next implementation slice should be an isolated verifier adapter and known-answer corpus, not immediate production dispatch wiring.
