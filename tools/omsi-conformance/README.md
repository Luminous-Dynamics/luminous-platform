# OMSI 0.1 — executable conformance seed

This is a deliberately small, reviewable reference validator for the OMSI pre-RFC. It validates JSON Schemas and selected semantic invariants for operation manifests, authority grants, lifecycle event sequences and evidence receipts.

## Run

```sh
python -m pip install -r tools/omsi-conformance/requirements.txt
make -C tools/omsi-conformance schemas
make -C tools/omsi-conformance test
python tools/omsi-conformance/omsi_validate.py operation tools/omsi-conformance/fixtures/operation.json --now 2026-10-12T12:05:00Z
python tools/omsi-conformance/omsi_validate.py grant tools/omsi-conformance/fixtures/grant.json --manifest tools/omsi-conformance/fixtures/operation.json --now 2026-10-12T12:05:00Z
python tools/omsi-conformance/omsi_validate.py receipt tools/omsi-conformance/fixtures/receipt.json --manifest tools/omsi-conformance/fixtures/operation.json
python tools/omsi-conformance/omsi_validate.py events tools/omsi-conformance/fixtures/events.json
```

## License

The files in this conformance seed are released under the MIT License. This license does not establish governance or licensing for the broader OMSI standard.

## Security boundary

This is not an executor, identity provider, authorization service or cryptographic verifier. It does not authenticate principals; verify proof/signature bytes; resolve issuer trust; check key revocation; provide replay persistence across processes; establish tenant isolation in deployment; prove declared verifier independence; or execute/rollback a host change. The grant signature is shape-checked only. Tests use deterministic sample data and do not prove production security.

A successful validator result means only that a document met these local schema and semantic checks. It must not be advertised as OMSI certification or complete MSP compliance.

## Intended next gate

Add an independently reviewed cryptographic grant-verification interface, durable nonce/idempotency state, HTTP integration tests, and a disposable-VM NixOS operation/recovery test before any real machine mutations or production claim.
