# Home system profile contract

The machine-readable profiles here describe proposed product configurations and their evidence ceiling. They are not product specifications, procurement approval, safety certification, or test results.

## Contents

- schemas/home-system-profile.schema.json — JSON Schema Draft 2020-12 product-profile contract.
- schemas/home-capability-contract.schema.json — versioned workload capability-request contract; it is a declaration, not owner authorization.
- schemas/home-effect-envelope.schema.json — Mycelix Hearth-bound request envelope; it cannot grant authority or prove physical effects.
- profiles/home-systems/effect-requests/ and profiles/home-systems/effect-invalid/ — positive and adversarial effect-envelope fixtures.
- docs/home-systems-myelix-hearth-integration.md — cross-project authority boundary, exact admission composition, privacy constraints and conformance ladder.
- profiles/home-systems/valid/ — positive profile fixtures, intentionally design-only.
- profiles/home-systems/invalid/ — product-profile adversarial fixtures expected to be rejected.
- profiles/home-systems/capabilities/ and profiles/home-systems/capability-invalid/ — one design-only workload contract and negative schema fixtures.
- tools/home-profile-validator/ — Rust schema and semantic contract validator; the shared production implementation is in src/lib.rs.
- tools/home-profile-validator/src/effect_protocol.rs — pure executable reference model for dispatch gating, replay, freshness and outcome reconciliation. This is not the production Hearth edge runtime. It binds an idempotency key to a canonical request fingerprint and makes unresolved outcomes reconciliation-only.
- docs/home-systems-prehardware-qualification-plan.md — staged pre-hardware checklist, VM lab design, failure-injection matrix, evidence bundle, and go/no-go gate.
- docs/home-systems-paradigm-shifts.md — research direction for a Sovereign Home Operating Environment, capability contracts, graceful degradation, owner exit, and evidence freshness.
- tests/home-systems-vm/ — initial NixOS/QEMU sentinel test of optional-inference failure isolation; test-double evidence only, not a Hearth implementation. Run it with a reviewed, pinned Nixpkgs checkout as documented in that directory.

## Local validation

With a Rust toolchain and network access to resolve crates on the first build:

```sh
cargo test --manifest-path tools/home-profile-validator/Cargo.toml
cargo run --manifest-path tools/home-profile-validator/Cargo.toml -- \
  schemas/home-system-profile.schema.json \
  profiles/home-systems/valid \
  profiles/home-systems/invalid
cargo run --manifest-path tools/home-profile-validator/Cargo.toml \
  --bin capability-contract-validator -- \
  schemas/home-capability-contract.schema.json \
  profiles/home-systems/capabilities \
  profiles/home-systems/capability-invalid
```

Run the effect-envelope fixture corpus with:

```sh
cargo run --manifest-path tools/home-profile-validator/Cargo.toml -- \
  schemas/home-effect-envelope.schema.json \
  profiles/home-systems/effect-requests \
  profiles/home-systems/effect-invalid
```

The GitHub Actions workflow repeats the Rust tests and all three schema/fixture checks when their inputs change.

## Required interpretation

- design_only profiles carry no implementation hash, hardware selection, frozen BOM, prototype, measured benchmark, qualification receipt, tested recovery, or non-zero current openness level.
- The schema can describe a future qualified profile, but the current Rust validator rejects every readiness=qualified claim. It has no trusted receipt-verification integration and cannot authenticate receipt references, independently hash external artifacts, or prove an experiment happened. This is intentionally fail-closed: add a separately reviewed verifier bound to a trusted evidence source before changing that behavior.
- A profile cannot override Mycelix Hearth physical-effect admission or grant Symthaea direct physical authority.
- robotics_lab and robotics_platform require an independent physical stop; qualification additionally requires that stop to be recorded as verified.
- The negative corpus is a required control. Weakening a rejection must be reviewed as a contract change, not treated as fixture maintenance.
- VM/simulation evidence must remain distinguishable from bench measurement and field evidence. A signed receipt authenticates provenance/integrity only to the extent its key and chain are trusted; it does not independently prove physical truth.

The initial positive fixtures intentionally contain no real hardware, product BOM, performance claims, or completed qualifications.


The Rust validator also refuses symlink traversal in fixture trees and prints the reasons that negative fixtures were rejected. Additional regression tests cover design-only hardware/recovery claims, openness promotion, robotics physical-stop policy, qualified capability promotion, and symlink traversal. These changes are not considered verified until the exact-head Rust workflow reports success.


The capability contract schema has Rust regression tests for the valid design-only example and negative controls for an empty egress allowlist and a critical physical effect missing authorization, postcondition observation, and independent stop safeguards. Schema conformance does not enforce the contract at runtime: the next implementation must compare the workload's requested capabilities to a separate owner-controlled policy and demonstrate that prohibited access is actually blocked.
