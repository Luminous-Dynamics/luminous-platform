# Community Connectivity Profile Validator

A small Rust CLI that validates the draft community-connectivity profile without contacting external schema references.

## Run

From the repository root:

```sh
cargo run --manifest-path tools/community-connectivity-profile-validator/Cargo.toml -- \
  schemas/community-connectivity-profile-v1.schema.json \
  profiles/community-connectivity-reference-v1.json
```

Run the unit tests:

```sh
cargo test --manifest-path tools/community-connectivity-profile-validator/Cargo.toml
cargo clippy --manifest-path tools/community-connectivity-profile-validator/Cargo.toml --all-targets -- -D warnings
```

The command prints a JSON report containing the exact schema/profile SHA-256 digests, validation scope, disposition, and errors. Exit codes: `0` valid draft contract, `1` invalid schema/profile policy, `2` I/O or parse/configuration failure, `64` usage error.

## Fail-closed guarantees

- Input size is bounded.
- Remote `$ref` and `$dynamicRef` schema references are rejected before constructing a validator; no URL is fetched.
- Draft 2020-12 meta-schema validation and format validation are enabled.
- The checked-in profile must match its strict schema, including its benchmark values, draft-only status, explicit non-claims, and exact hard gates.
- Policy checks independently require that Starlink's South African service authorization and plan-sharing permission remain `not_verified`, deployment stays blocked pending evidence, and its standard WAN path does not require a private vendor API.
- Negative tests mutate draft status, Starlink authorization fields, benchmark targets, non-claim declarations, and schema references to verify rejection.

## Limits

This validator checks a machine-readable design contract only. A successful exit does **not** establish that:
- a measurement actually occurred or is physically true;
- a radio, terminal, gateway or firmware is qualified;
- the BOM is complete, hardware source artifacts are truly open, or supply is reproducible;
- any network or spectrum activity is authorized;
- a Starlink plan or provider permits community sharing/resale;
- the design is production-ready or meets its proposed performance/economic targets.

Keep VM, bench and field evidence distinct. Physical test results must be recorded in the existing hardware evidence/qualification process and independently reviewed according to its requirements. CI, build, test, Clippy, Cargo.lock generation and physical tests are still required before integration or qualification; no runner result is implied by these source files.
