# Vendor Adapter Contract and Evidence Packet v1

**Status:** implementation contract proposal; not yet enforced by executable code  
**Scope:** global, vendor-neutral inventory and lifecycle platform  
**Parent:** [Issue #23](https://github.com/Luminous-Dynamics/luminous-platform/issues/23)  
**Architecture:** [Global vendor interoperability and firmware services](GLOBAL_VENDOR_INTEROPERABILITY_AND_FIRMWARE_SERVICES_V1.md)

## Decision

Implement the adapter boundary before implementing vendor-specific mutations. Every adapter must declare its scope and capabilities, return typed outcomes, and emit evidence that can be independently checked. Unknown capability, model, revision, firmware or regional behavior is `unsupported`—never an implicit fallback.

## 1. Adapter manifest

Each adapter release should publish a machine-readable manifest with at least:

- `schema_version`, `adapter_id`, `adapter_version`, source commit and build/artifact digest.
- Vendor, product family, exact supported model/SKU/revision constraints, firmware/software ranges and regional/cloud endpoint constraints.
- Capability entries, each with operation, risk class, read/write classification, reversibility, required permissions, prerequisite evidence, known limitations and last-verified date.
- Credential type and minimum required scope; rate limits and retry semantics; supported API/protocol/CLI versions.
- Recovery prerequisites, backup limits, maintenance window requirements and escalation route.
- Qualification level (A0–A5) with links to exact-head test receipts and physical-device test evidence. Mocks alone cannot establish physical qualification.

Do not use wildcard support such as “all Cisco” or “all NETGEAR” as a qualification claim. Wildcards may express discovery hints, not permission to mutate.

## 2. Common operation envelope

Every request is bound to a unique operation ID and exact target identity. A mutation request additionally includes:

- Target identity and observed model/revision/software state.
- Canonical desired-state diff and digest.
- Pre-state digest and observation time/freshness limit.
- Actor, customer/tenant, policy/profile revision and authorization receipt.
- Expiry, maintenance window and operation-specific idempotency key.
- Risk classification, recovery plan reference and expected service impact.

Canonicalization must be versioned. The digest is computed over the canonical bytes, not an implementation-dependent debug representation. Credentials and secret values must never be included in the operation envelope or digest payload.

## 3. Typed outcome model

Use a closed, versioned result enum:

- `succeeded_verified`: operation and required postconditions independently observed.
- `succeeded_unverified`: device/API acknowledged the operation but required postconditions were not observed; never treat as qualified success.
- `rejected_precondition`: target, model, revision, pre-state, authorization or policy does not match.
- `unsupported`: exact target/capability is outside the adapter manifest.
- `failed_no_effect`: evidence supports that no side effect occurred.
- `indeterminate`: side effect may have occurred; do not blindly retry.
- `partial`: some intended effects occurred; report exact observed subset and invoke the approved recovery policy.
- `rate_limited` / `temporarily_unavailable`: retry only according to operation-specific semantics and deadline.

A transport timeout is not proof of failure. A successful API response is not proof of device state. Replays must consult the durable effect journal and fresh device state.

## 4. Minimal evidence packet

An evidence packet should contain:

- Packet schema version and unique receipt ID.
- Target pseudonymous ID plus exact vendor/model/revision/software facts required to scope the claim.
- Adapter ID/version, source commit, build digest and capability manifest digest.
- Request/action digest, pre-state digest and post-state digest where observed.
- Authorization receipt ID/digest, policy/profile revision, actor identity and expiry.
- Start/end times, freshness challenge/nonce and clock-quality caveat.
- Raw evidence references and cryptographic digests; raw sensitive artifacts remain in customer-controlled encrypted storage.
- Executor result and independently generated verifier result as separate identities/records.
- Test suite/version, test outcomes, exceptions and qualification scope.
- Signature algorithm/key identifier and signature; key status/revocation evidence at verification time.
- Supersedes/disputes links and explicit limitations.

A valid signature proves control of a signing key over bytes; it does not prove that the underlying statement is true. A digest proves equality with a referenced artifact, not the artifact's provenance or meaning.

## 5. Authorization and local fail-safe behavior

Before any write operation, the local enforcement service must check:
1. Actor and tenant authorization are valid and unexpired.
2. The target identity is exact and in scope.
3. Current pre-state matches the approved pre-state; otherwise reject and require replanning.
4. The canonical action digest matches the approved digest.
5. Adapter manifest explicitly permits the operation for this target and software revision.
6. Required backup/recovery prerequisites are satisfied.
7. Local policy permits the operation during the current maintenance window.
8. Required independent approval is present for the risk class.

If a check is unavailable, behavior is explicitly defined by operation class. No new high-risk mutation proceeds on missing authorization, stale state, unknown capability or unavailable required verifier. Existing device policy and local forwarding must not depend on the distributed ledger being online.

## 6. Required contract tests

### Schema and canonicalization
- Missing required field, unknown enum, wrong schema version and malformed digest fail closed.
- Equivalent input encodings canonicalize identically; ambiguous/duplicate keys are rejected.
- Secret-like fields are rejected from public packet serialization.

### Target and authority binding
- Wrong tenant, wrong device, model/revision mismatch, expired authorization and revoked signer are rejected.
- Action digest or pre-state digest mutation invalidates approval.
- Stale observations and replayed nonce/operation IDs are rejected or resolved through the journal.

### Failure and replay
- Timeout before request transmission; timeout after device applies; duplicate delivery; partial commit; rate limit; API outage; device reboot mid-operation.
- Indeterminate outcomes are never auto-replayed unless the operation is proven idempotent and fresh state is re-read.
- Rollback is a new authorized operation unless the operation's approved recovery procedure explicitly binds it.

### Evidence integrity
- Tampered raw artifact, signature, signer key status, receipt linkage or verifier identity is detected.
- Executor cannot provide its own independent-verifier identity.
- Conflicting evidence, supersession, revocation and offline verification have deterministic outcomes.
- Holochain/DHT unavailability does not bypass local checks or make a pending change authorized.

### Vendor contract fixtures
- Missing fields, changed API versions, unexpected pagination, rate limiting, wrong regional endpoint, revoked credentials and vendor error payloads.
- CLI output parsing, if needed, is bounded to pinned command templates and tested parser fixtures; arbitrary command execution is not an adapter capability.

## 7. Qualification levels

- **A0 Discovered:** source/API documentation and target scope recorded.
- **A1 Read-only tested:** inventory/export observed on exact lab hardware and software.
- **A2 Contract tested:** schemas, preconditions, authorization, journal and failure/retry semantics tested.
- **A3 Change qualified:** specific low-risk mutation, postcondition and recovery tested on exact hardware.
- **A4 Firmware/recovery qualified:** exact image lifecycle and recovery path tested on physical hardware.
- **A5 Serviceable:** regional sourcing/support, entitlement, field procedure, customer contract and escalation are established.

Qualification is per adapter capability and target scope, not a blanket score for a vendor. The current document is a proposal; no adapter has been promoted by this document alone.

## 8. First implementation slice

1. Implement JSON Schema for adapter manifest, operation envelope and evidence packet.
2. Add a deterministic canonicalizer and digest fixture tests.
3. Add a fake adapter with no write capability and fixture-driven outcomes.
4. Add independent packet verifier with tamper, replay, stale-state and signer-revocation tests.
5. Only then connect one real device in read-only mode and record exact model/revision/software plus test evidence.
6. Require exact-head automated results and independent evidence review before changing qualification status.

## Non-goals

- Universal vendor compatibility.
- Arbitrary shell/CLI execution.
- Firmware writes in the first implementation slice.
- Treating Holochain record validity as proof of physical truth.
- Publicly storing credentials, full configs or sensitive customer topology.
- Claiming certification, production readiness or vendor authorization from schema or mock tests alone.
