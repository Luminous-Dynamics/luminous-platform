# Community Hardware Receipt Verifier

Rust-first, offline verifier for `luminous.hardware-test-receipt.v1` records.

## Scope

- Validates schema documents against the JSON Schema Draft 2020-12 meta-schema.
- Validates receipts against the committed schema with format validation enabled.
- Rejects remote schema references so untrusted schemas cannot cause network retrieval.
- Checks evidence-class/execution-mode consistency, disposition/result consistency, exact artifact-digest references, privacy markings, physical-claim boundaries, and measurement-instrument calibration references.
- Optionally verifies public artifact bytes in a local directory named by lowercase SHA-256 digest.
- Emits a machine-readable JSON report and nonzero exit status on invalid receipts.

It does **not** certify hardware, establish that a claimed run occurred, authenticate a reviewer, grant deployment authority, promote H0-H5 maturity, fetch URLs, modify a host, or flash firmware. A valid receipt is a well-formed evidence submission, not proof that its assertions are true.

## Run

From the repository root:

```sh
cargo run --manifest-path tools/community-hardware-receipt-verifier/Cargo.toml -- \
  schemas/community-hardware-test-receipt-v1.schema.json \
  docs/fixtures/community-hardware-test-receipt-v1.json
```

To check artifact bytes, supply a local directory containing public artifacts as files whose names are their declared 64-character SHA-256 digests:

```sh
cargo run --manifest-path tools/community-hardware-receipt-verifier/Cargo.toml -- \
  schemas/community-hardware-test-receipt-v1.schema.json \
  docs/fixtures/community-hardware-test-receipt-v1.json \
  --artifact-root ./artifacts
```

Remote `retrieval_uri` values are never fetched.

## Tests

```sh
cargo test --manifest-path tools/community-hardware-receipt-verifier/Cargo.toml
cargo clippy --manifest-path tools/community-hardware-receipt-verifier/Cargo.toml --all-targets -- -D warnings
```

Direct dependencies are exact-version pinned in `Cargo.toml`. Commit a reviewed `Cargo.lock` before integrating this crate into automated release/qualification workflows. The current authoring environment did not have Rust/Cargo installed, so compilation, lockfile generation and test execution remain outstanding.
