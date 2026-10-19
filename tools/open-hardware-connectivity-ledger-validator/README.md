# Open Hardware Connectivity Ledger Validator

Offline Rust validator for the draft connectivity hardware openness ledger. It runs Draft 2020-12 JSON Schema validation, then locks the profile identity, candidate IDs, all “design candidate only” statuses, critical deployment gates, no-self-promotion claim rules and required follow-up evidence.

## Run

    cargo test --manifest-path tools/open-hardware-connectivity-ledger-validator/Cargo.toml --all-targets
    cargo clippy --manifest-path tools/open-hardware-connectivity-ledger-validator/Cargo.toml --all-targets -- -D warnings
The CLI expects these inputs from the repository root:

    cargo run --manifest-path tools/open-hardware-connectivity-ledger-validator/Cargo.toml -- schemas/open-hardware-connectivity-candidate-ledger-v1.schema.json profiles/open-hardware-connectivity-candidate-ledger-v1.json

The last command emits JSON with exact schema/profile SHA-256 digests and an explicit statement that validation conveys no hardware openness, device qualification or deployment authorization. Exit codes are 0 for valid, 1 for validation failure, 2 for input/validator errors and 64 for incorrect arguments.

## What is enforced

- The only candidate IDs in v1 are GL-MT6000 and Banana Pi BPI-R4 research candidates.
- Both remain design candidates only; public deployment and all regional/provider gates remain unverified.
- A firmware download, design-file link, OpenWrt support entry, CI pass or Starlink IP compatibility cannot promote the full device to open hardware.
- Each candidate has an artifact-level status matrix. Unknown, closed/blob-dependent, partial, and documented-but-untested states remain explicit.
- External schema references are rejected before validator construction; the tool does not fetch network resources.
- Required next artifacts and claim gates cannot be silently removed.

## Critical non-claims

A successful validation proves only schema and draft policy consistency. It does not confirm that cited source files exist or are licensed as expected, reproduce a firmware image, establish upstream support for the exact purchased revision, test radio drivers, prove type approval, or authorize ISP sharing/resale. Only a separate exact-device evidence receipt and review may support a scoped qualification claim.

The device ledger is intended to make trade-offs visible, not to declare a fully open product before its firmware, radio calibration, BOM, board files and licence chain have been reviewed.
