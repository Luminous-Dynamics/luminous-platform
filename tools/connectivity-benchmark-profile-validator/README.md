# Connectivity benchmark profile validator

Offline, fail-closed structural and draft-policy validation for
`profiles/connectivity-broker-benchmark-profile-v1.json`.

## Run

From the repository root, in a Rust-enabled environment:

```sh
cargo test --manifest-path tools/connectivity-benchmark-profile-validator/Cargo.toml
cargo run --manifest-path tools/connectivity-benchmark-profile-validator/Cargo.toml -- \
  schemas/connectivity-broker-benchmark-profile-v1.schema.json \
  profiles/connectivity-broker-benchmark-profile-v1.json
cargo clippy --manifest-path tools/connectivity-benchmark-profile-validator/Cargo.toml --all-targets -- -D warnings
```

The CLI emits a JSON report with SHA-256 digests of the exact schema/profile bytes.
Exit status is 0 for a valid profile, 1 for validation errors, 2 for file/JSON or
validator-construction failures, and 64 for incorrect command-line arguments.

## Safety and evidence boundary

- Input files are bounded; the tool does not fetch network resources.
- Schema references must be local fragments. Remote `$ref` and `$dynamicRef`
  values are rejected before validator construction.
- JSON Schema structure is supplemented by semantic checks for profile identity,
  immutable comparison rules, all 16 benchmark identities and their target/status
  pairs, required non-claims, and required follow-up artifacts.
- A successful result means only that the document conforms to this versioned
  draft contract. It does not prove that any measurement is true, the code is
  compiled or tested, a network works, hardware is qualified, any provider permits
  resale, a radio deployment is authorized, or a service is production-ready.
- All thresholds remain proposed targets or blocked/baseline-first items. The
  validator intentionally does not promote these to measured results.

The authoring environment may not have Rust/Cargo installed. Unless a workflow
report binds to the exact commit SHA, tests must be reported as **authored, not run**.
