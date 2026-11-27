# WIMSE + AIP + OAuth Transaction Token Interoperability Profile v0.1

**Status:** Proposed Luminous relying-party profile and conformance plan. This is not an adopted IETF profile, deployed integration or qualification result.

**Tracking:** Platform #549 (AIP interop tests), #544 (source-anchored issuer / commit-time authorization), #546 (canonical action binding), #547 (authenticated exact descendant task/plan pairs), and ATR PR #542.

## 1. Protocol roles: do not make one token do everything

The interoperable design separates four credentials/proofs from the policy decision and the effect receipt:

1. **Workload Identity Token (WIT)** — establishes the calling workload identity and its holder key.
2. **Workload Proof Token (WPT)** — proves possession of that key for a request to a specific audience and can bind the WIT and other context tokens by hash.
3. **Transaction Token (Txn-Token)** — conveys the immutable subject, transaction identifier, narrowly defined scope/purpose and contextual attributes across an internal call chain.
4. **Delegation chain (AIP or equivalent)** — conveys who delegated what authority to which agent/workload, with signed attenuation and bounded depth.
5. **Relying-party authorization decision** — combines workload identity, source policy P0, principal entitlements, exact delegation branch, transaction context, approval/consent and current revocation/fencing state.
6. **Participant effect receipt** — states whether the exact protected-resource operation committed, was absent, conflicted or remains unknown.

These are not interchangeable:
- A valid WIT does not prove that a particular operation is authorized.
- A valid WPT proves possession of a workload key under its specified request-binding profile, not that the request is allowed by P0.
- A Txn-Token binds transaction context but does not by itself prove caller workload identity.
- An AIP chain proves delegated authority lineage, not presenter identity or a protected-resource commit.
- A PDP allow decision does not prove the PEP or participant committed the exact action.
- A signed completion claim is not an independent effect receipt.

## 2. External specifications and their status

### 2.1 WIMSE Workload Identity and Management Architecture

The current draft draft-ietf-wimse-aims-01 describes identity, credentials, authorization, lifecycle and monitoring for AI-agent workloads. It also discusses service-to-service delegation and transaction-scoped credentials.

Important AIMS guidance:
- A local agent UI confirmation is not itself authorization. The confirmation must be bound to a verifiable authorization grant; the final authorization decision remains with the authorization server.
- It discusses OAuth Transaction Tokens for tool-to-service access, reducing the practice of forwarding broad access tokens across internal services.
- It recommends monitoring and security-event integration so that lifecycle and authorization changes can be reconstructed.

AIMS is an Internet-Draft, not an adopted standard. Its own text notes that mid-execution approval requires further design beyond basic CIBA initiation.

Reference: https://datatracker.ietf.org/doc/draft-ietf-wimse-aims/

### 2.2 WIMSE Workload Proof Token

The current draft draft-ietf-wimse-wpt-02 proposes a WPT conveyed in the HTTP Authorization header, signed by the private key corresponding to the holder key in the WIT.

Its validation requirements include:
- validate the WIT and verify that the WPT signature uses the confirmation key in the WIT;
- require exactly one Authorization header and a single well-formed WPT JWT;
- check the WPT media type and the request target URI against the token audience, using trusted authority configuration rather than an untrusted Host or X-Forwarded-Host header;
- require a valid expiration time and reject an unreasonable far-future expiry;
- bind the WPT to the WIT with the wth hash claim;
- optionally check jti reuse within the token-validity window (the draft RECOMMENDS this);
- when present, bind a Txn-Token using tth, and end-user/authorization context tokens using oth;
- reject unknown token-hash entries and do not use extra context tokens not covered by oth for authorization.

The WPT intentionally excludes Bearer credentials from the same Authorization header. The draft warns that honoring bearer credentials alongside WPT would reduce security to the weakest credential.

Limitations:
- The jti replay check is a RECOMMENDATION, not an unconditional MUST in the cited validation list.
- Binding to a TLS endpoint/channel is optional and its mechanisms are out of scope.
- The draft explicitly warns that WPT does not protect major portions of request/response data and does not protect against an actively malicious middlebox. Do not treat WPT as a full HTTP request signature.
- It is an Internet-Draft, not an adopted standard.

Reference: https://datatracker.ietf.org/doc/draft-ietf-wimse-wpt/

### 2.3 OAuth Transaction Tokens

The current draft draft-ietf-oauth-transaction-tokens-11 describes short-lived signed tokens that convey immutable authorization context across a call chain. Its claims include:
- iat, aud, exp, unique txn, sub, and a narrowly defined scope;
- recommended tctx for immutable transaction context;
- recommended rctx for environmental context;
- req_wl to identify the requesting workload.

The Transaction Token Service (TTS) validates the subject token, chooses a trust-domain audience, sets the transaction ID and times, and evaluates the requested scope. It is authoritative for the context claims it issues. The draft expects a distinct logical TTS per trust domain.

The draft explicitly says a Txn-Token is not an access token for arbitrary use; it is a context carrier for the specific transaction across an internal call chain. Use it to prevent an internal tool/service from needing the agent's broad inbound access token.

The draft's authorization policy for whether a workload may obtain a transaction token remains deployment-defined. A Txn-Token does not solve principal entitlements or source-policy compilation by itself.

Reference: https://datatracker.ietf.org/doc/draft-ietf-oauth-transaction-tokens/

### 2.4 AIP delegation chain

AIP draft -02 provides a delegation chain and JWT/Biscuit encodings, but its own text reports gaps in proof-of-possession implementation, V7 revocation, principal entitlement composition (R6) and execution-time approval (R10).

Reference: https://datatracker.ietf.org/doc/draft-prakash-aip/

AIP and WPT solve different problems: AIP models delegation; WPT binds the request presenter to a workload key. Txn-Token binds transaction context. None replaces source P0 or participant commit-time enforcement.

## 3. Proposed call envelope

A high-impact request should carry or securely resolve:
- WIT and WPT for the executing workload;
- AIP / delegated capability chain, with exact parent and root task/plan pair bindings;
- Txn-Token in its dedicated header, not Authorization;
- source policy P0 digest and policy generation;
- principal entitlement and tenant context;
- exact canonical action digest using the existing Act-Bound profile;
- explicit approval/consent authorization event if required;
- current revocation/fencing epoch;
- one-use or bounded-replay invocation nonce;
- exact participant resource/operation binding.

The WPT wth, tth and oth bindings should be validated as specified by the current draft. A Luminous deployment profile should additionally require a participant-side replay cache / idempotency record for consequential mutations, rather than relying only on the WPT draft's RECOMMENDED jti reuse check.

The production action digest must use the pinned Act-Bound profile: strict vr-jcs = 0.4.1 raw JSON admission / RFC 8785 canonicalization and domain-separated BLAKE3 with context org.luminous-dynamics.act-finality.v1. Do not reuse the CAG fixture's SHA-256 digest as a production action token.

## 4. Required verifier order

1. **Strict transport/header parsing:** reject duplicate Authorization, duplicate or malformed token headers, ambiguous target normalization, invalid JSON/duplicate keys at the action adapter, and unsupported content transformations.
2. **Workload identity:** validate the WIT issuer/audience/lifetime and its holder key.
3. **Proof of possession:** validate WPT signature against the WIT confirmation key; bind audience to trusted target configuration; verify wth; verify tth and oth if corresponding context tokens are consumed; check jti replay under the Luminous mandatory profile.
4. **Delegation:** validate AIP chain order/signatures, root binding, depth, exact attenuation, branch task/plan pair, delegation context and principal/delegate identity binding.
5. **Transaction context:** validate Txn-Token signature, trust-domain audience, iat/exp, txn, sub, scope, req_wl, and required immutable tctx values. Confirm the requesting workload is authorized to obtain and use this transaction context.
6. **Source authorization:** check exact operation/arguments/resource/audience against immutable P0 and principal entitlements. AIP or Txn-Token cannot expand P0.
7. **Human approval:** where required, verify a fresh authorization-server grant / approval event bound to the exact action digest. Local UI state alone does not authorize the effect.
8. **Commit-time fence:** recheck credential/key/grant revocation generation and participant fencing epoch at the protected-resource commit barrier.
9. **Independent finality:** record the participant transaction ID/state revision, then verify the effect from an independent observer/receipt source. Response loss enters reconciliation, not blind retry.

Unknown or conflicting state never falls through to ALLOW.

## 5. Security test matrix

| Test family | Negative control | Required result |
|---|---|---|
| WIT/WPT key binding | WPT signed by wrong key | Reject |
| WPT audience | Wrong aud, forged Host/X-Forwarded-Host | Reject |
| WIT binding | Incorrect wth | Reject |
| Transaction binding | Incorrect tth | Reject |
| User context binding | Incorrect/unknown oth entry | Reject or ignore only per explicit profile; never authorize from unbound token |
| Replay | Reuse jti inside validity period | Reject under Luminous profile |
| Header downgrade | Add Bearer token alongside WPT | Reject duplicate/weakest-credential downgrade |
| AIP truncation | Present valid parent JWT prefix without leaf PoP | Reject |
| AIP identity | cnf key doesn't match agent/workload ID | Reject |
| AIP revocation | Stale or unavailable V7 response | Fail closed for high impact |
| Txn scope | Same transaction, changed recipient/resource/amount | Reject at participant |
| Principal entitlement | Agent chain allows but root principal not entitled | Reject |
| Approval | UI clicked but no grant; stale/wrong-call/replayed grant | Require fresh bound approval or deny |
| Commit-time revocation | Grant revoked after planning, before commit | Reject at commit |
| Middleware mutation | Request body/authority changed by intermediary outside PoP | Detect via strict action digest or end-to-end signed request profile |
| Unknown result | Response lost after commit | Reconcile original transaction; no new mutation |
| Effect provenance | Agent says success; participant has no matching commit receipt | Not verified |

## 6. Benchmark outputs

Preserve protocol-native conformance results separately from ATR:
- WIT validation;
- WPT proof-of-possession and audience validation;
- replay rejection and replay-cache availability;
- wth/tth/oth token binding;
- AIP chain, attenuation, depth, identity and revocation checks;
- Txn-Token signature, audience, lifetime, transaction and context validation;
- principal entitlement result;
- human approval result;
- participant commit-time fence result;
- independent effect receipt result;
- p50/p95/p99 validation, revocation propagation and reconciliation latency;
- required-call block rate and task completion.

Hard security failures are non-compensable. Report transaction context binding, presenter identity, delegation validity, source authorization, approval and effect finality as separate predicates.

## 7. Scope limitations

- These are evolving Internet-Drafts. Their publication does not imply IETF adoption, universal implementation or independent security certification.
- WPT's jti replay checking is only RECOMMENDED in the current draft; a Luminous high-impact profile should strengthen it to a deployment MUST with a shared replay store and fail-closed behavior.
- WPT doesn't sign all request/response bytes and warns about malicious middleboxes. For high-risk requests, use an end-to-end request-signature profile or a trusted TLS termination and action-digest verifier.
- AIMS notes local UI confirmation isn't authorization, but CIBA's basic client-initiation model does not fully specify mid-execution approval.
- Txn-Token issuance policy remains deployment-defined; the TTS must be independently protected.
- AIP -02 has gaps in PoP, V7, R6 and R10. The relying party still needs P0 and participant finality.
- A valid token does not prove the protected resource committed. Independent effect receipts remain mandatory.

## 8. References

- WIMSE AIMS draft -01: https://datatracker.ietf.org/doc/draft-ietf-wimse-aims/
- WIMSE WPT draft -02: https://datatracker.ietf.org/doc/draft-ietf-wimse-wpt/
- OAuth Transaction Tokens draft -11: https://datatracker.ietf.org/doc/draft-ietf-oauth-transaction-tokens/
- AIP draft -02: https://datatracker.ietf.org/doc/draft-prakash-aip/
- Luminous Act-Bound Crypto Profile v0.2: https://github.com/Luminous-Dynamics/luminous-platform/blob/main/research/agent-effect-gate-v0.1/ACT_BOUND_CRYPTO_PROFILE_V0.2.md

