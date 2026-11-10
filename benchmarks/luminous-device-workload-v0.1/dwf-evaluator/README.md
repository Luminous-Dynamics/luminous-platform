# DWF-001 independent placement evaluator

**Status:** standalone Rust source and synthetic-fixture evaluator; not a production scheduler or authorization service.

This crate implements an independent, deterministic evaluator for the 30-case DWF-001 placement and evidence scenario corpus. It deliberately lives outside a repository-wide Cargo workspace so that it does not create a new workspace/build boundary for Luminous Platform.

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


### Final fixture coverage in this tranche

The 30-case corpus includes:
- DWF-001-23: incomplete execution receipt rejection;
- DWF-001-24: complete receipt positive control;
- DWF-001-25: invalid executor signature rejection;
- DWF-001-26: request-owned runtime digest positive control;
- DWF-001-27: wrong/unselected executor rejection;
- DWF-001-28: oracle output-commitment mismatch rejection.


## Input contract validation

Before any expected disposition is evaluated, the evaluator validates the fixture envelope: schema version, unique scenario/candidate IDs, exact origin/peer roles, request semantic identity, non-empty ISA, positive minimum memory/deadline/energy ceiling, read-only/no-external-effect constraints, typed policy and transfer-budget fields, network state, and result-phase oracle-verifier identity. This prevents missing requirements from silently becoming unconstrained defaults. The current 30-scenario fixture has an independent structural cross-check from the authoring environment. The 33 Rust source tests include a deterministic single-field hard-gate tightening matrix, a non-compensatory privacy/performance control, candidate-order permutation checks, canonical scenario-report ordering under fixture permutation, a test-only execution lifecycle model, and adversarial event-order/recovery checks. All 23 tests still require Cargo execution. Deliberately incomplete execution receipts remain valid *negative-control inputs* and are evaluated as rejected results; they are not rejected as malformed fixture envelopes.


The accepted network-state vocabulary includes explicit `disconnected` as well as connected, pre-dispatch partition, post-dispatch partition and reconnected states. A disconnected peer is ineligible before dispatch; the local origin may remain eligible.


### Device-profile and posture binding

The result receipt must now bind to the selected peer's device-profile digest, posture digest and exact board revision. DWF-001-29 rejects a receipt from the correct peer ID but a different device-profile digest; DWF-001-30 rejects a receipt tied to a different posture snapshot. These values are fixture assertions, not cryptographic attestation appraisal.


The 30-case corpus also binds result receipts to the exact selected peer device-profile digest, posture digest and board revision. The incomplete-receipt negative control includes missing device/posture claims as explicit rejection reasons.


### Adversarial and metamorphic test coverage

The evaluator's table-driven hard-eligibility test starts from an eligible peer and applies one constraint-tightening mutation at a time. Mutations cover local-only policy, destination allowlists, source data classification, peer session scopes and revocation, lease expiry, local/destination policy denial, stale posture, unknown device profile, ISA and memory mismatch, runtime/workload digest mismatch, backend mismatch, deadline and energy overruns, missing/exceeded transfer budgets, disconnection, non-read-only workload class and external effects. Every mutated peer must become ineligible.

A separate non-compensatory test gives a data-scope-denied peer unrealistically favorable latency, energy and thermal values and verifies that the privacy gate still wins. Candidate assessments are sorted by stable candidate ID, and permutation tests verify that reversing candidate input order does not change the selected Pareto frontier or assessment ordering. Scenario reports are likewise sorted by stable scenario ID: reversing the fixture array preserves the canonical per-case report while the fixture SHA-256 correctly changes because the exact input bytes changed.

These are deterministic source-level tests, not executed Rust test results. They do not replace property-based fuzzing, cryptographic receipt verification, independent review or physical qualification.


### Deterministic execution-lifecycle reference model

A test-only state machine now models the read-only request lifecycle across authorization, dispatch, execution, indeterminate completion, receipt persistence, independent oracle verification and acceptance. It is deliberately not a production workflow runtime.

The model exercises:
- authorization revocation before dispatch;
- partitions, lost acknowledgements and reconnects that remain INDETERMINATE;
- acceptance only after the current attempt has a durable receipt and a passing oracle verdict;
- duplicate-safe retry gating, with retries blocked after revocation;
- delayed receipts from an earlier attempt, which cannot satisfy a newer attempt;
- origin restart, which discards unpersisted in-memory completion observations;
- explicit reconciliation of a durable receipt after restart, followed by separate oracle verification;
- all 24 orderings of partition, reconnect, lost acknowledgement and origin restart, plus all 120 orderings that add revocation; no new attempt can start under the revoked grant, and no acceptance occurs before receipt persistence and oracle verification;
- deterministic replay of an identical event schedule.

The model counts attempts so duplicate read-only work is visible rather than silently treated as exactly-once execution. Its event outcomes are synthetic model evidence only. It does not implement network transport, durable storage, cryptographic receipt verification, real revocation propagation or a production retry controller.
