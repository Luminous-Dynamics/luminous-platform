# Community Wi-Fi roaming profile validator

A small offline Rust crate for validating the Passpoint/OpenRoaming research profile and an evidence-bearing interoperability matrix. The CLI runs the Draft 2020-12 JSON Schema validator and then enforces versioned policy invariants that should not be silently weakened.

## Run tests

From the repository root with Rust and Cargo installed:

```sh
cargo test --manifest-path tools/community-wifi-roaming-profile-validator/Cargo.toml --all-targets
cargo clippy --manifest-path tools/community-wifi-roaming-profile-validator/Cargo.toml --all-targets -- -D warnings
```

Validate the checked-in profile and blank matrix template:

```sh
cargo run --manifest-path tools/community-wifi-roaming-profile-validator/Cargo.toml -- \
  schemas/community-wifi-roaming-benchmark-profile-v1.schema.json \
  profiles/community-wifi-roaming-benchmark-profile-v1.json \
  schemas/community-wifi-roaming-interoperability-matrix-v1.schema.json \
  profiles/community-wifi-roaming-interoperability-matrix-template-v1.json
```

The CLI emits JSON with SHA-256 digests for all four input files, the benchmark count, row count, errors and explicit non-claims. Exit codes are 0 valid, 1 validation failure, 2 input/validator error, and 64 usage error.

## Policy enforced

- Exact profile identity/version and exact target/operator/status tuples for all 13 benchmark IDs.
- Federation, identity, upstream sharing, regulatory and equipment approval states remain unverified/not obtained; deployment stays blocked.
- Credential validation remains fail-closed; local-only service does not require WAN; packet contents are not inspected for routing.
- All comparison guards, non-claims and next required artifacts remain present.
- Empty matrix template state must match an empty row list; once rows are present, status must change to the evidence-matrix state.
- Matrix rows require exact claim scope, per-client/profile/AP/AAA/federation/device/OS information, limitations and an explicit evidence class.
- Tested claims require a timestamp, evidence class, receipt path and receipt SHA-256. Federated OpenRoaming supported/unsupported claims require both named test credential state and verified authorized test enrollment.
- Raw trial denominators must equal succeeded + failed + inconclusive; trial IDs must be unique; untested rows cannot claim trials.
- Schema remote references are rejected before validator construction.

## Evidence boundary

This validator makes no device, OS, AP, AAA, identity-provider, federation or field-performance claim. A valid profile is not an enrolled federation. A valid matrix row only applies to the exact recorded combination and receipt; a schema-valid receipt is not proof that measurements are true or representative. Unknown stays unknown.

Do not include passwords, private keys, EAP inner identities, RADIUS secrets, raw packet contents, browsing history or direct personal identifiers in public profile/matrix data. Use synthetic credentials in tests and pseudonymous site/network tokens in evidence.

## Research sources

- WBA OpenRoaming overview: https://wballiance.com/openroaming/how-it-works/
- WBA OpenRoaming portal source (tooling only, not membership): https://github.com/wireless-broadband-alliance/wba-openroaming-portal
- ICASA services licensing: https://www.icasa.org.za/pages/services-licencing
- ICASA licensing/type approval: https://www.icasa.org.za/pages/licensing
