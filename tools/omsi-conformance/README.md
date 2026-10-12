# OMSI 0.1 — executable conformance seed

This is a deliberately small, reviewable reference validator for the OMSI pre-RFC. It validates JSON Schemas and selected semantic invariants for operation manifests, authority grants, lifecycle event sequences, evidence receipts and a prototype OMSI-to-Nixward identity-binding record.

## Run

```sh
python -m pip install -r tools/omsi-conformance/requirements.txt
make -C tools/omsi-conformance schemas
make -C tools/omsi-conformance test
make -C tools/omsi-conformance validate-fixtures
make -C tools/omsi-conformance interop
python tools/omsi-conformance/omsi_validate.py operation tools/omsi-conformance/fixtures/operation.json --now 2026-10-12T12:05:00Z
python tools/omsi-conformance/omsi_validate.py grant tools/omsi-conformance/fixtures/grant.json --manifest tools/omsi-conformance/fixtures/operation.json --now 2026-10-12T12:05:00Z
python tools/omsi-conformance/omsi_validate.py receipt tools/omsi-conformance/fixtures/receipt.json --manifest tools/omsi-conformance/fixtures/operation.json --events tools/omsi-conformance/fixtures/events.json
python tools/omsi-conformance/omsi_validate.py events tools/omsi-conformance/fixtures/events.json --manifest tools/omsi-conformance/fixtures/operation.json
```

## License

The files in this conformance seed are released under the MIT License. This license does not establish governance or licensing for the broader OMSI standard.

## Canonicalization profile

See [CANONICALIZATION.md](CANONICALIZATION.md) for the exact prototype framing and registered digest domains. Action-argument and operation-manifest digests are intentionally domain-separated and are not interchangeable.

The seed accepts a deliberately restricted JSON value set for digest-bearing contract documents: duplicate object keys, negative zero, floating-point numbers, unsafe-range integers and strings/keys that cannot be represented as valid Unicode scalar values are rejected. Object member names are ordered by UTF-16 code units; arrays retain order; canonical bytes are hashed with SHA-256 using an explicit semantic domain prefix. This is a constrained accepted-input profile designed to avoid ambiguous cross-language encodings, **not a claim of full RFC 8785/JCS conformance**. The exact algorithm and test vectors must receive independent cross-language review before a stable protocol version. The `interop` target uses a small independent JavaScript implementation to check fixed basic/Unicode vectors and the action/manifest/grant/receipt digest bindings; it is a prototype, not a second fully conforming implementation.

## Nixward identity-binding prototype

The pure helper in `nixward_adapter.py` binds an already validated OMSI operation manifest to the plan digest, target-machine digest and transaction ID carried by one versioned Nixward SystemTransaction envelope. It refuses unsupported envelope schema/version values and malformed digest shapes. Its schema keeps the OMSI SHA-256 manifest digest tagged separately from Nixward BLAKE3 identities. The helper fails closed on manifest, tenant, resource, correlation or supplied transaction-identity mismatch.

This is only a deterministic identity-binding contract. It does not create or verify Nixward ChangePlan/ChangeAuthorization objects, call `NixOSExecutor::execute_authorized()`, persist nonces, attest the source of Nixward digest arguments, observe a real machine, or verify recovery. Callers must pass the authoritative Nixward transaction envelope being used, not synthesize a lookalike object or copy values back out of an untrusted binding record. This Python checker still cannot prove the envelope was produced by Nixward's Rust implementation. Passing the unit tests is not a Nixward integration or VM qualification result.

## Security boundary

The prototype only accepts `secret_refs` and `credential_refs` as arrays of opaque URI-shaped references, and rejects whitespace, userinfo and query-bearing reference strings. This is structural hygiene, not discovery of every possible secret and not proof that the referenced vault/resource is trustworthy. Raw secret material still belongs in a separate secret-delivery mechanism.

This is not an executor, identity provider, authorization service or cryptographic verifier. It does not authenticate principals; verify proof/signature bytes; resolve issuer trust; check key revocation; provide replay persistence across processes; establish tenant isolation in deployment; prove declared verifier independence; or execute/rollback a host change. The grant signature is shape-checked only. Tests use deterministic sample data and do not prove production security.

When `receipt` validation is supplied both `--manifest` and `--events`, a verified receipt is cross-checked against the operation-bound lifecycle sequence: the post-state observation cannot predate application, the terminal `verified` event cannot predate the observation, and the receipt cannot predate that event. Without `--events`, the validator can only enforce chronology fields present in the receipt and manifest; it cannot infer when execution occurred. These checks validate declared timestamps and event structure, not signed provenance or actual host state.

A successful validator result means only that a document met these local schema and semantic checks. It must not be advertised as OMSI certification or complete MSP compliance.

## Intended next gate

Add an independently reviewed cryptographic grant-verification interface, durable nonce/idempotency state, HTTP integration tests, wire the binding to Nixward's real authorized-execution result, and run the disposable-VM NixOS operation/recovery test before any real machine mutations or production claim.
