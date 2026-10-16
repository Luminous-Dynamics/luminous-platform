# Firmware assurance corpus checker

This is a small Rust-first tool for **Stage 0 corpus integrity**, not a firmware verifier.

It checks that the synthetic benchmark corpus:
- parses as JSON and validates against the supplied JSON Schema;
- contains the declared number of cases with sequential, unique IDs;
- uses one exact synthetic profile;
- keeps case kind and expected disposition aligned;
- keeps positive, negative and indeterminate fixtures semantically distinguishable;
- requires explicit read-only assertions in every case;
- computes each case's stage disposition from a versioned subset of required claim states: any required refutation blocks; otherwise any unknown/not-applicable requirement is indeterminate; only all-verified requirements are eligible for synthetic lab staging;
- runs deterministic mutation tests against this small policy composer;
- remains bound to exact corpus/schema bytes by SHA-256 in its deterministic report.

## Usage

From the repository root:

```sh
cargo test --manifest-path tools/firmware-assurance-corpus-checker/Cargo.toml

cargo run --manifest-path tools/firmware-assurance-corpus-checker/Cargo.toml -- \
  validate-corpus \
  docs/fixtures/firmware-assurance-corpus-v1.json \
  docs/schemas/firmware-assurance-corpus-v1.schema.json
```

The command exits with:
- `0`: the corpus schema and cross-field contract passed;
- `1`: the corpus was parsed but is blocked by schema or contract violations;
- `2`: malformed/unreadable inputs, an unusable schema, or invalid command usage.

Inputs are each capped at 4 MiB and the corpus is capped at 10,000 cases. The report intentionally emits no wall-clock timestamp so equal inputs produce byte-stable semantic output.

## Claim ceiling

A passed report means only that the **test corpus is schema/contract-valid and its declared claim-state combinations agree with the simple synthetic stage policy**. It does not execute the prose-described attacks or verify a real UKI, verify a firmware signature, establish key authorization/revocation, prove hardware identity, assess OEM compatibility, authorize an update, or qualify rollback/recovery. The expected claim states are test inputs, not facts discovered from a device.

The next stage must implement executable case inputs and a separate oracle/verifier before any firmware-level correctness claim can be made. Do not feed these corpus descriptions into an update executor.
