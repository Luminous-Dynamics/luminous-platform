# OMSI 0.1 — executable conformance seed

This is a deliberately small, reviewable reference validator for the OMSI pre-RFC. It validates JSON Schemas and selected semantic invariants for operation manifests, authority grants, lifecycle event sequences and evidence receipts.

## Run

```sh
python -m pip install -r tools/omsi-conformance/requirements.txt
make -C tools/omsi-conformance schemas
make -C tools/omsi-conformance test
make -C tools/omsi-conformance validate-fixtures
python tools/omsi-conformance/omsi_validate.py operation tools/omsi-conformance/fixtures/operation.json --now 2026-10-12T12:05:00Z
python tools/omsi-conformance/omsi_validate.py grant tools/omsi-conformance/fixtures/grant.json --manifest tools/omsi-conformance/fixtures/operation.json --now 2026-10-12T12:05:00Z
python tools/omsi-conformance/omsi_validate.py receipt tools/omsi-conformance/fixtures/receipt.json --manifest tools/omsi-conformance/fixtures/operation.json
python tools/omsi-conformance/omsi_validate.py events tools/omsi-conformance/fixtures/events.json --manifest tools/omsi-conformance/fixtures/operation.json
```

## License

The files in this conformance seed are released under the MIT License. This license does not establish governance or licensing for the broader OMSI standard.

## Canonicalization profile

See [CANONICALIZATION.md](CANONICALIZATION.md) for the exact prototype framing and registered digest domains. Action-argument and operation-manifest digests are intentionally domain-separated and are not interchangeable.

The seed accepts a deliberately restricted JSON value set for digest-bearing contract documents: duplicate object keys, negative zero, floating-point numbers, unsafe-range integers and strings/keys that cannot be represented as valid Unicode scalar values are rejected. Object member names are ordered by UTF-16 code units; arrays retain order; canonical bytes are hashed with SHA-256 using an explicit semantic domain prefix. This is a constrained accepted-input profile designed to avoid ambiguous cross-language encodings, **not a claim of full RFC 8785/JCS conformance**. The exact algorithm and test vectors must receive independent cross-language review before a stable protocol version.

## Security boundary

The prototype only accepts `secret_refs` and `credential_refs` as arrays of opaque URI-shaped references, and rejects whitespace, userinfo and query-bearing reference strings. This is structural hygiene, not discovery of every possible secret and not proof that the referenced vault/resource is trustworthy. Raw secret material still belongs in a separate secret-delivery mechanism.

This is not an executor, identity provider, authorization service or cryptographic verifier. It does not authenticate principals; verify proof/signature bytes; resolve issuer trust; check key revocation; provide replay persistence across processes; establish tenant isolation in deployment; prove declared verifier independence; or execute/rollback a host change. The grant signature is shape-checked only. Tests use deterministic sample data and do not prove production security.

A successful validator result means only that a document met these local schema and semantic checks. It must not be advertised as OMSI certification or complete MSP compliance.

## Intended next gate

Add an independently reviewed cryptographic grant-verification interface, durable nonce/idempotency state, HTTP integration tests, and a disposable-VM NixOS operation/recovery test before any real machine mutations or production claim.
