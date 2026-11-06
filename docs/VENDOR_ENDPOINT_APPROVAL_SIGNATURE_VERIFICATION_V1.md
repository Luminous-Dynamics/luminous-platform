# Vendor Endpoint Approval Signature Verification v1

**State:** offline signature-verification contract; not a production authorization service.

## Signed-message contract

The verifier accepts only the versioned scheme `ed25519-rfc8032-luminous-approval-v1`, using Ed25519 as specified by [RFC 8032](https://www.rfc-editor.org/rfc/rfc8032). The signed bytes begin with the domain-separation prefix `LUMINOUS:VENDOR-MANAGEMENT-ENDPOINT-APPROVAL:V1` followed by exactly one NUL byte, then UTF-8 JSON serialized with sorted object keys, no insignificant whitespace, no duplicate object members, no floating-point numbers, and no non-finite values. The JSON payload includes the complete decision record, including the decision state, target/profile/adapter bindings, approver and key references, allowed operations, policy revision, and validity interval. Only `signed_payload_digest` and `signature_evidence_ref` are excluded to avoid a circular reference. The SHA-256 of those complete signed bytes must equal `signed_payload_digest`.

The signature is a detached raw 64-byte Ed25519 signature. `signature_evidence_ref` must equal `sha256:` plus the lowercase SHA-256 digest of that exact 64-byte file. The key reference and algorithm/scheme are themselves covered by the signed payload. A changed payload, key, signature, or reference fails closed.

## Trust-store contract

The trust store is supplied separately and validated against [the trust-store schema](../schemas/vendor-management-approval-trust-store-v1.schema.json). It maps an approval principal to an explicitly delegated Ed25519 public-key reference and the `endpoint_approval_signer` role. Key and principal validity use half-open UTC-normalized intervals `[valid_from, valid_until)`. A recorded revocation takes effect at `revoked_at`; verification checks that timestamp against the current execution time.

The caller must pass `--trust-store-sha256` from an independently protected configuration channel. The CLI also requires `--capability-registry-sha256` from a protected channel; a digest copied from the approval bundle or a pull request is not an independent trust pin. Do not compute the only trusted digest from a trust-store file delivered alongside an untrusted approval or PR. The verifier checks the raw bytes of the trust-store file against that external digest before parsing it. No private key or permanent synthetic trust root is committed.

Example invocation:

```sh
python tools/verify_vendor_management_endpoint_approval_signature.py \
  --approval /protected/evidence/approval.json \
  --profile /protected/evidence/frozen-profile.json \
  --signature-file /protected/evidence/approval.sig \
  --trust-store /protected/policy/vendor-approval-trust-store.json \
  --trust-store-sha256 "$VENDOR_APPROVAL_TRUST_STORE_SHA256" \
  --capability-registry security/vendor-adapter-capability-registry-v1.json \
  --capability-registry-sha256 "$VENDOR_CAPABILITY_REGISTRY_SHA256"
```

The shell variable above must come from a protected policy channel; it is not a value to copy from the approval bundle.

## Exact capability-scope gate

Before reporting a successful signature check, the CLI validates the capability registry and requires its exact raw-byte SHA-256 to match the external `--capability-registry-sha256` pin. Every requested operation must be marked `tested` for the exact adapter source revision and exact model, product family, hardware revision, software version, and assessed hardware/commercial region. The adapter must have at least A1 read-only maturity, exact-model-tested scope, and `LAB_TESTED` or `QUALIFIED` implementation state. Missing, planned, duplicate, mismatched, or unqualified operations fail closed.

This regional scope is the product/hardware/commercial region recorded in the endpoint profile. It is not inferred from deployment jurisdiction or from the management API hostname; those remain separate decision dimensions. The checked-in registry currently leaves all operations planned and all adapters at A0, so it must reject all real operation requests until exact test evidence is reviewed and the independently pinned registry changes.

## What this verifier does not establish

A successful result establishes signature authenticity, signer/key trust against the digest-pinned store, current key/principal validity and revocation timing, structural/profile binding, and the decision's own time window. It explicitly prints that **operational authorization is not evaluated**. It does not verify a fresh independently observed inventory record against `target.identity_digest`, evaluate a complete newer-decision/revocation ledger, establish jurisdictional or procurement authority beyond the supplied profile, or execute a device operation. A production caller must combine it with a separately trusted inventory observation, a complete decision-revocation/supersession check, exact tested capability scope, and a protected execution policy before considering any operation.

The test suite generates ephemeral synthetic keys in memory and exercises tampering, pin mismatch, principal/key delegation, signer/key revocation and time-window cases. It does not qualify any real signer, key, device, or endpoint.
