# Connectivity benchmark validators

This package contains two independent offline CLIs:

- `connectivity-benchmark-profile-validator`: validates the versioned benchmark target profile.
- `connectivity-measurement-receipt-verify`: validates a draft measurement receipt against the exact benchmark-profile bytes, checks its trial denominators, and hashes its local evidence artifacts.

## Run

From the repository root, using a Rust-enabled environment:

```sh
cargo test --manifest-path tools/connectivity-benchmark-profile-validator/Cargo.toml --all-targets
cargo clippy --manifest-path tools/connectivity-benchmark-profile-validator/Cargo.toml --all-targets -- -D warnings
```

The profile validator CLI is:

```sh
cargo run --manifest-path tools/connectivity-benchmark-profile-validator/Cargo.toml --bin connectivity-benchmark-profile-validator -- \
  schemas/connectivity-broker-benchmark-profile-v1.schema.json \
  profiles/connectivity-broker-benchmark-profile-v1.json
```

The receipt verifier CLI is:

```sh
cargo run --manifest-path tools/connectivity-benchmark-profile-validator/Cargo.toml --bin connectivity-measurement-receipt-verify -- \
  schemas/connectivity-measurement-receipt-v1.schema.json \
  path/to/receipt.json \
  profiles/connectivity-broker-benchmark-profile-v1.json \
  path/to/evidence-root
```

The receipt's artifact paths must be relative to the supplied evidence root. The verifier recomputes their lengths and SHA-256 digests. The referenced profile SHA-256 is calculated over the exact bytes of the supplied profile file; the receipt's benchmark ID, operator, target and target status must match that exact profile record.

## Measurement receipt contract

Each receipt records one named benchmark run with:
- repository commit, config digest, optional firmware-image digest, build metadata and profile digest;
- explicit source-static, VM-simulated, bench-measured or field-observed evidence class;
- site token and hardware metadata at the required evidence level;
- raw per-trial outcomes, attempted/succeeded/failed/inconclusive counts and measurement method;
- local evidence artifact paths, file sizes and SHA-256 values;
- privacy assertions and explicit limitations.

The verifier rejects duplicate trial IDs, inconsistent denominators, target drift from the referenced benchmark profile, evidence-class/environment mismatches, a missing or altered config artifact, unsafe artifact paths, symlink artifacts, incorrect byte lengths and SHA-256 mismatches. Remote schema references are rejected before validation to avoid network retrieval.

## Critical non-claims

A valid receipt demonstrates structural conformity and matching local bytes—not measurement truth, independent review, field representativeness, service quality, provider permissions, equipment qualification, spectrum rights or deployment authorization. A receipt remains `draft_unreviewed`; independent review should be a separate, attributable attestation rather than an edit that silently promotes the original.

Field evidence requires pseudonymous site identification, named gateway hardware/revision, firmware-image digest, collection window and device count. Do not include packet bodies, message content, subscriber identifiers, credentials, exact household locations or raw browsing histories in public evidence artifacts.

Do not write to a running evidence directory during verification. Preserve the receipt, profile, config and raw-trace bytes; corrections should create a new receipt and new artifact digests rather than rewriting the old record.

No test or CLI run is claimed unless a Rust-enabled result is recorded against the exact commit SHA. In this authoring session Rust/Cargo are unavailable, so the unit tests are authored but unexecuted.
