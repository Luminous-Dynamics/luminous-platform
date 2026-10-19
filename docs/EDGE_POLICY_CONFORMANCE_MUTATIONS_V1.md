# Edge policy conformance mutation suite v1

**Status:** deterministic synthetic validator-stress test. It does not test a device, network path, firewall efficacy, or attacker-detection performance.

## Paradigm shift

A frozen set of known negative cases is necessary but incomplete. A policy contract should survive systematic adversarial changes to authority, verdicts, identifiers, evidence requirements, schema boundaries and privacy fields. The suite mutates one aspect of a fresh base corpus per trial and requires the validator to reject every deliberately invalid variant.

The runner reuses a precompiled JSON Schema validator. It cycles through 24 mutation operators to guarantee coverage, while a seeded PRNG chooses case indexes and mutation parameters. The report is deterministic for the same exact schema bytes, corpus bytes, count and seed. It contains only aggregate counts, digest-bound identities, per-operator totals and up to eight digests of accepted mutants; mutated inputs are not emitted.

## Run

\`\`\`sh
cargo run --manifest-path tools/edge-policy-conformance-mutations/Cargo.toml --bin edge-policy-conformance-mutations -- --count 10000 --seed 20261019
\`\`\`

The CLI exits non-zero if any deliberately malformed or semantically contradictory mutation is accepted. Default run: 10,000 trials; maximum requested count: 100,000. The suite exercises a synthetic JSON contract, not a real data plane.

## Mutation families

- Stale/revoked/unknown authority changed to allow; deny changed to permitted; positive allow changed to blocked.
- Independent probe flag/evidence removed.
- Duplicate IDs and invariant items; missing required fields; invalid ID format.
- Unknown subscriber/device identity fields and unsupported enums.
- Negative or wrong-type response-time values, too few cases, wrong schema version, and altered evidence/claim classes.

These mutations cover distinct schema-level and semantic guards. They are not a representative malware corpus, do not prove completeness against every possible policy bug, and do not substitute for a mature property-based testing framework or formal methods.

## Proposed acceptance targets

| Metric | Initial target |
|---|---|
| Seeded invalid inputs | At least 10,000 per frozen schema+corpus revision |
| Invalid mutant acceptance | 0; one accepted critical mutation fails the run |
| Operator coverage | 100% of registered mutation operators covered in every full run |
| Reproducibility | Identical mutation digest for identical schema, corpus, seed and count |
| Seed sensitivity | Different seeds should change the generated mutation digest |
| Base false rejection | 0; the unmodified positive corpus must validate before any mutation runs |
| Auditability | Every report binds exact schema and corpus byte digests, count, seed, operator totals and overall mutation digest |

These are engineering acceptance goals for the contract validator, not claims of production security or industry certification. The CI profile executes 10,000 trials at seed 20261019 plus 20 additional fixed seeds (20261020–20261039) at 1,000 trials each, for 30,000 mutation trials per run. Preserve failed seeds and first accepted-mutant digests for debugging.

## State-of-the-art comparison plan

- Retain the exact schema/contract baseline and compare mutation-acceptance deltas by version rather than only total test count.
- Use mature property-based/fuzzing tools for input parsers after the first deterministic contract corpus; these mutations are a reproducible regression layer, not a replacement for fuzzing.
- Cross-check intended reachability with Batfish where supported; use Cilium/Hubble observations only for Cilium-managed traffic; retain independent external probes for wider physical-network enforcement.
- Keep measured network results separate from this benchmark. The actual namespace allow/deny run lives in the separate [ephemeral namespace lab work](https://github.com/Luminous-Dynamics/luminous-platform/pull/83).

## Evidence ceiling

A passing run says the validator rejected the registered seeded corpus of invalid synthetic mutations. It does not authenticate receipts, prove the validator is bug-free, quantify field attack resistance, or qualify Meraki/Palo Alto/Luminous security outcomes.

