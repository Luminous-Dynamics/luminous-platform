# Sovereign Compute Benchmark v0.1

This directory defines a versioned evidence-binding contract for matched sovereign-compute benchmark runs, as tracked in Platform issue [#370](https://github.com/Luminous-Dynamics/luminous-platform/issues/370).

## Files

- `benchmark-run.schema.json`: JSON Schema 2020-12 for one complete benchmark run manifest.
- `example-modelled-run.json`: deliberately modelled/synthetic example showing exact workload, source, policy, environment, arms, outcomes, metric boundaries and artifact hashes.

## Run identity and evidence

A benchmark result is meaningful only when its task semantics, workload bytes, policy, candidate systems, source commits, hardware, runtime, measurement boundaries and evidence class are bound together. The manifest keeps separate:
- the benchmark protocol revision;
- exact workload and expected-output manifest digests;
- policy digest and owner decision boundaries;
- environment nodes and evidence quality;
- each competitor's own exact source commit and configuration digest;
- accepted/rejected/deferred/indeterminate/unknown outcomes;
- raw observation digests and explicit measurement boundaries;
- evidence class and claim limitations.

The schema intentionally does not define a combined benchmark score. Performance, correctness, privacy, resilience, autonomy and owner economics remain separate scoreboards.

## Evidence classes

- `synthetic`: generated inputs or outcomes.
- `modelled`: simulation or model output.
- `loopback`: real processes over loopback networking.
- `emulated_network`: real processes under a controlled emulated network.
- `physical_lan`: independently instrumented physical devices over a local network.
- `field`: owner-authorized field deployment under a declared threat and workload profile.

A row cannot be promoted from one class to another by inference. The node-specific evidence quality is a separate property.

## Validation

Validate the manifest against JSON Schema 2020-12 using a standards-compliant validator. Structural validation does not authenticate artifacts, verify cryptographic claims, establish actual measurement integrity, enforce count relationships, or prove that a result is correct.

The producer/independent auditor must additionally verify:
1. each artifact URI resolves to the exact bytes with the declared SHA-256;
2. source commits and build configuration are immutable and available;
3. the declared workload and expected-output manifests match the exact benchmark semantics;
4. count invariants hold (for example, `correct_accepted <= accepted <= executed <= dispatched <= offered`, subject to declared semantics);
5. every metric's raw observations support the reported statistic and measurement boundary;
6. no `synthetic` or `modelled` observation is relabelled `physical_lan` or `field`;
7. comparisons declare policy/semantic mismatches instead of hiding them;
8. the result and verification strategy comply with owner data-access and authority policy.

## Non-goals

This is a manifest schema, not a scheduler, attestation verifier, cryptographic signature verifier, evidence ledger, HDC oracle, hardware measurement system, privacy protocol or benchmark result.
