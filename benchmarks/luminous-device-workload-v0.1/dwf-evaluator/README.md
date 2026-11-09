# DWF-001 independent placement evaluator

**Status:** standalone Rust source and synthetic-fixture evaluator; not a production scheduler or authorization service.

This crate implements an independent, deterministic evaluator for the 28-case DWF-001 placement and evidence scenario corpus. It deliberately lives outside a repository-wide Cargo workspace so that it does not create a new workspace/build boundary for Luminous Platform.

## Run

From this directory:

~~~sh
cargo test
cargo run -- ../fixtures/placement-scenarios-v0.1.json
~~~

The default fixture path is `../fixtures/placement-scenarios-v0.1.json`. The CLI emits a versioned JSON report containing the exact fixture SHA-256, per-case disposition, expected-disposition comparison and candidate gate reasons. Exit status is 0 only when every scenario matches; malformed or unreadable input exits 2; mismatches exit 1.

## Evaluator rules

1. Hard eligibility precedes optimization. Unknown or stale capability, runtime, posture, session, authorization, resource or transfer facts do not become eligible defaults.
2. Local-only policy blocks all peer egress. A peer must be explicitly permitted for the destination and carry both a read-only compute scope and a matching data-class scope.
3. Derived representations inherit the source data classification by default. HDC/BinaryHV query vectors, projected features, top-k IDs/distances and diagnostic traces are not presumed anonymous or public. Declassification requires a separate, explicit, policy-authorized determination outside this evaluator.
4. ISA, exact runtime/workload digests, required backend, memory, latency deadline, energy budget and (where specified) thermal headroom are hard gates.
5. If no candidate satisfies every hard gate, the disposition is ABSTAIN / NOT_DISPATCHED. This includes the case where both local and remote devices exceed the energy budget.
6. Eligible candidates are compared as a Pareto set over latency, energy and thermal headroom. No opaque scalar score trades privacy, correctness or eligibility for speed.
7. A result is accepted only when its execution receipt contains dispatch/completion, fresh posture, observed backend, exact runtime digest, valid signature, output digest and an explicit passing independent-oracle outcome. Missing claims reject the result; a complete receipt with a valid signature and passing oracle is accepted; an invalid signature or failed oracle is rejected. Missing acknowledgements or durable receipts remain INDETERMINATE; no success is inferred.

## Evidence ceiling

The fixture contains synthetic scenarios and synthetic resource observations. A passing evaluator test means only that this implementation agrees with the expected dispositions encoded in that fixture. It does **not** establish that the expected outcomes are correct, that a real device is trusted, that remote execution is safe, that network isolation exists, or that any physical performance/energy advantage has been measured.

This is a pure evaluator for the fixture contract. It does not connect to devices, Xenia, Mycelix, Symthaea, EnergyMeter, a network, a task runtime or an effect executor. It does not authorize dispatch or consequential actions. Independent review and exact-head Rust tests/CI remain required.


### Runtime identity source of truth

The expected runtime digest is taken from the frozen workload requirements in the request, never from an execution-receipt field. The receipt's runtime digest is compared to that request-owned value. DWF-001-26 is the positive control: it accepts a complete receipt without duplicating an `expected_runtime_digest` field inside the execution receipt.

**Fixture-only verification limit:** `signature_valid` and `oracle_result` are synthetic fixture outcomes. This evaluator does not cryptographically verify a result signature, authenticate an oracle, run the HDC oracle, appraise device attestation or authorize dispatch. A production result-verifier must independently verify the signer and oracle evidence against trusted, request-bound identities and digests.


### Request and oracle binding

Result validation now binds the executor receipt to the selected peer, request-owned runtime/workload digests, input commitment, required correctness-oracle ID and required verifier ID. The oracle receipt must bind the exact request input and output digest. DWF-001-27 rejects a result from a peer that was not selected; DWF-001-28 rejects an oracle receipt for a different output.

These verifier/signature booleans and IDs are synthetic fixture assertions only. The evaluator checks that the contract fields exist and agree; it does not cryptographically authenticate the executor or oracle, appraise the device, or run the HDC oracle. That production work is DWF-RESULT-001 #358.
