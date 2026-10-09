# Cooperative Commerce Core (Rust)

**Status: proposed M0 calculation kernel.** This crate is a small, dependency-free Rust library for exact-decimal landed-cost comparisons. It is not a purchasing service, evidence verifier, tax engine, payment system, or production-qualified integration.

## Why this is Rust-first

The arithmetic and comparability rules belong in a reusable Rust library rather than in a Python production service or a user-interface calculator. The existing Python declaration validators remain prototype/CI tools; this crate is the native calculation kernel intended for later integration behind reviewed Rust APIs.

## Implemented contract

- Parses base-10 decimal strings without exponent notation or whitespace; never accepts binary floating-point inputs.
- Uses checked i128 arithmetic and a bounded decimal scale. Overflow or unsupported precision returns an error rather than rounding silently.
- Requires the baseline and actual basket to contain the exact same product identifier, specification digest, unit code, unit-code system, and quantity. It does not infer substitutions or perform unit conversions.
- Requires explicit, referenced SHA-256-shaped evidence records for each line and cost. The crate checks their structure, not their content or signer.
- Requires the same landed-cost category set for baseline and actual purchases. Every category must have cost lines or an explicit zero-confirmation on each side; a category cannot be both charged and confirmed zero. Evidence-backed credits/rebates reduce totals, and unresolved material costs block the result.
- Includes baseline costs, actual delivery/other costs, and participation fees in the net result.
- Refuses mixed currencies. FX conversion needs a separately reviewed rate record with source, timestamp, purpose, and rounding rules; no implicit conversion occurs.
- Preserves negative net differences rather than clipping them to zero.
- Binds delivery receipts to the exact product identities, unit systems, and quantities being compared; a partial or mismatched delivery blocks the full-basket comparison.
- Distinguishes estimates, pure invoice-vs-quote comparisons, historical-invoice comparisons, mixed invoice/quote baselines, and calculations without delivery evidence. A cost-coverage summary document does not itself determine the comparison's evidence class.
- Returns evidence_authenticated_by_calculator=false unconditionally. It does not validate code-list membership, authenticity, legal compliance, or transaction authority.

## Portable receipt

The proposed [Savings Receipt schema](../../schemas/cooperative-savings-receipt-v1.schema.json) defines the source snapshot, itemized inputs, calculation fields, claim class, and explicit limitation that the calculator has not authenticated evidence. An illustrative fixture lives in the cooperative-buying pack. This crate currently returns Rust structs; JSON receipt serialization and independent re-verification remain separate work.

## Formula

**net_difference = baseline_merchandise + baseline_other_costs − actual_merchandise − actual_other_costs − participation_costs**

Each line amount is already the total for the exact comparable quantity. The calculator does not multiply unit prices or estimate omitted costs. Those costs must be represented as explicit, evidence-linked cost lines; unresolved charges block calculation.

A positive result is a computed difference, not proof that the platform caused the difference or that all source documents are genuine. A public list-price baseline is always labeled an estimate. An invoice comparison without delivery evidence is provisional.


## Effective-dated code-list lookup

The `code_list` module provides a dependency-free Rust index over **already-normalized** records. It strictly parses Gregorian `YYYY-MM-DD` dates, requires unambiguous validity windows, permits a code to recur only across non-overlapping historical periods, performs exact case-sensitive lookups, and fails closed if the snapshot itself is outside its effective window. Regression tests cover boundary dates, overlapping historical entries, malformed dates, empty values, and unknown/ineffective codes.

This is a record-semantics layer, not an importer or source-authenticity system. It does not read SIX/UNECE source files, compute source/payload hashes, verify signatures or publisher authority, determine review freshness, or enable registry activation. Callers must complete those independent checks before constructing a runtime index. No official source payload is bundled or claimed as verified.


## SIX ISO 4217 List One importer

The dependency-free `six-iso4217-import` binary parses a **retained local** copy of SIX's current List One XML and emits deterministic normalized JSON. It preserves numeric codes as strings, aggregates repeated currency codes across entities, accounts for entries without a currency tuple, and fails on partial or conflicting tuples. It rejects DTDs and user-defined/external entities, malformed dates, mismatched tags, duplicate fields, and other unsupported XML constructs.

Example:

```sh
cargo run --locked --manifest-path crates/cooperative-commerce-core/Cargo.toml --bin six-iso4217-import -- ./retained/list-one.xml > ./normalized/list-one.json
sha256sum ./retained/list-one.xml ./normalized/list-one.json
```

`publication_date` reflects the XML `Pblshd` attribute; it is not interpreted as an effective date. Retain the original bytes, exact source URL, retrieval timestamp, source and normalized payload hashes, and importer source/build digest alongside the output. The tool does not fetch or authenticate the source, verify a signature, evaluate license terms, or activate a registry. Tests use synthetic XML only; no official SIX payload is bundled or claimed tested.

## Tests

From the repository root:

```sh
cargo fmt --manifest-path crates/cooperative-commerce-core/Cargo.toml -- --check
cargo test --locked --manifest-path crates/cooperative-commerce-core/Cargo.toml
```

The crate also includes a typed savings-receipt model and a verifier that recomputes every reported field from the included typed input snapshot and rejects mismatched claimed totals. A computed receipt requires source-revision and implementation-digest metadata; an illustrative fixture cannot claim computed status. This verifier checks arithmetic/provenance metadata only, not authenticity or signatures. JSON/JCS receipt serialization and independent external evidence verification remain separate work.

The crate has no third-party dependencies. CI must still pass on the exact PR head before these rules are treated as tested in the target environment.
