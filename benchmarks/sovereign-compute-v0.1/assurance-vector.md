# DWF assurance-vector contract v0.1

## Purpose

A distributed-compute candidate should not advertise one opaque "trust score." It should expose a structured, evidence-bearing **guarantee profile** describing which properties it provides, which it assumes, who verifies them, and what leaks or fails. The requesting owner supplies hard constraints. The placement policy admits only candidates whose profiles meet those constraints; optimization happens only within the feasible set.

This is a research architecture proposal, not an implemented production contract.

## 1. Work contract

Each request binds the following before dispatch:

| Contract area | Required fields |
|---|---|
| Intent | workload identity, semantic version, purpose, exact input digest(s), output schema, deadline |
| Semantics | exact or approximate; distance/task definition; deterministic tie/result policy; acceptable quality threshold |
| Authority | data owner, resource owner, grant/lease epoch, session identity, purpose, allowed destination and operation scope, expiry and revocation handle |
| Information flow | input/query/corpus/derived-data/result classes; authorized readers; permitted egress; output-release policy; metadata exposure |
| Correctness | independent oracle/verifier identity and version, verification relation, receipt binding, false-accept policy, result freshness |
| Confidentiality | protected objects, adversary roles, protocol, crypto parameters, TEE firmware/appraisal assumptions, MPC collusion threshold, ORAM trust assumptions |
| Resource | CPU/GPU/memory/storage/network limits, energy ceiling, thermal/battery reserve, local QoS floor and owner withdrawal policy |
| Recovery | idempotency class, duplicate execution contract, attempt identity, indeterminate state, durable receipt and reconciliation procedure |

The request must be signed/authorized by the relevant owner authorities in the production implementation. This document only defines the fields conceptually; it does not create a signer or identity service.

## 2. Guarantee vector

For each execution candidate, record a profile over distinct dimensions:

`G = (A, Q, D, P, O, I, V, R, L, E)`

- `A`: authority and scope correctness.
- `Q`: query/input confidentiality.
- `D`: corpus and derived-data confidentiality.
- `P`: access-pattern and search-pattern privacy.
- `O`: output/minimal-disclosure privacy.
- `I`: execution integrity / resistance to malicious substitution.
- `V`: independent result verifiability and freshness.
- `R`: resilience, revocation and correct reconciliation.
- `L`: local autonomy / offline-safe operation.
- `E`: owner resource and externality compliance.

Each dimension is multi-valued, not a boolean:

- `not_claimed`: no guarantee is asserted.
- `policy_declared`: policy/configuration asserts it, but no enforcement evidence is attached.
- `mechanism_enforced`: an identified mechanism enforces the stated property under named assumptions.
- `independently_verified`: a separate verifier has checked the property for this workload/run and evidence class.
- `failed`: a known violation or negative result exists.
- `unknown`: evidence is absent or contradictory.

Every non-trivial claim also names:
1. the protected object or decision;
2. the adversary/observer role;
3. assumptions and excluded attackers;
4. mechanism and exact implementation/configuration;
5. verifier and verification relation;
6. evidence artifact digest and evidence class;
7. residual leakage/failure modes.

Do not infer one coordinate from another:
- encryption does not imply access-pattern obliviousness;
- TEE attestation does not prove correct computation;
- a valid signature does not prove the signed result is true;
- a correct result does not prove the data access was authorized;
- local recomputation is unavailable when the origin is not authorized to read peer-resident inputs;
- a successful retry does not erase duplicate-execution or uncertain-attempt cost.

## 3. Hard admission, then Pareto placement

Admission is a predicate, not a weighted score:

`eligible(candidate, request) = all_required_guarantees_satisfied && all_hard_resource_constraints_satisfied`

- `unknown` does not satisfy a required property.
- `failed` always rejects that candidate for the affected requirement.
- A policy-required guarantee must be backed by a mechanism and the minimum declared evidence class.
- Resource owner reserves and withdrawal policy are hard constraints, not soft utility terms.
- Result correctness and authorization are separate gates.
- If any outcome is `INDETERMINATE`, no new mutation/retry is admitted until the exact attempt and durable receipt are reconciled under current authority.

Only after hard admission should the scheduler compare latency, energy, bandwidth, verification cost, owner externality and availability. Keep the feasible Pareto frontier. Owner preference can choose among feasible candidates; it cannot waive a hard constraint or make unknown evidence true.

Do not compress `G` into a "trust score" or "privacy score". Report the coordinate vector and evidence for every candidate.

## 4. Example capability profiles (hypotheses to verify, not automatic guarantees)

| Execution mode | Typical strong point | Typical residual/weak point |
|---|---|---|
| Consented plaintext peer | Low execution overhead; simple debugging | Peer sees query/data; requires explicit disclosure consent |
| SAP / DCPE | Low-cost transformed/perturbed search | Similarity/geometry leakage; not equivalent to cryptographic confidentiality |
| EMVP / FHE flat search | Can protect query and stored vectors under cryptographic assumptions | Cost, memory/bandwidth; no automatic malicious-server correctness |
| BNTM | Adds a specific malicious-server verification relation | Verifier and protocol cost; not a general-purpose proof of arbitrary execution |
| Tiptoe / PIR-LWE | Can hide which index cluster is queried | High client/server work and recall/scale trade-offs |
| Hierarchical FHE ANN | Encrypted arithmetic and practical candidate reduction | Selected cluster/leaf access patterns, output and repeated-query metadata may leak |
| TEE | Protects enclave memory from some host adversaries | Side channels, page/access traces, rollback/debug, hardware/firmware trust |
| ORAM / BiSON-like | Hides data access patterns under stated ORAM assumptions | Client state, bandwidth, stash/position-map and setup assumptions |
| Doubly-oblivious / O2N2-like | Stronger access privacy without small trusted-storage assumption | Circuit ORAM / oblivious-selection overhead and ANN-specific semantics |
| MPC / threshold | Hides inputs from individual parties within collusion threshold | Interactive rounds, dropout and operator-independence assumptions |

These are families, not certifications. For a given build and workload, claims can be weaker or stronger than the table suggests; measure and verify the actual implementation.

## 5. Exact HDC example

For Symthaea BinaryHV:
- input query: 16,384 bits / 2,048 bytes;
- peer corpus: exact digest of frozen vector bytes and stable IDs;
- task: exact Hamming distance `popcount(q XOR x)`, exact top-k, deterministic lowest-stable-ID tie-break;
- privacy choices: plaintext consent, TEE, exact encrypted evaluation, or a separately implemented oblivious-access protocol;
- result choices: IDs only, IDs plus distances, top-1, top-k, or full distance profile;
- correctness choices: origin recomputation only when authorized, diverse re-execution under explicit independence assumptions, or a concrete proof/verification relation;
- acceptance binds query digest, index revision, execution version, attempt ID, receipt digest and current oracle result.

Changing to approximate ANN changes the task semantics and moves the run to DWF-ANN-PRIVATE; it does not count as an exact-HDC win.

## 6. Composition boundaries

- Platform owns work contracts, policy admission, placement and attempt/reconciliation control.
- Mycelix/Xenia own recognized identity, owner authority/session consent and attestation/transport integration according to their existing contracts.
- Symthaea owns computation semantics and exact HDC result definitions.
- Symtropy owns measured or predicted device/energy/thermal resource evidence.
- A verifier is separately identified; it never inherits owner authority merely because it can inspect a result.
- Sol-Atlas may contribute spatial context where location/topology matters, but does not become an identity or policy authority.

No duplicate scheduler, signer, attestation verifier, transport, task runtime or evidence ledger is introduced by this design.

## 7. Benchmarking implications

Every benchmark arm should expose:
- guarantee vector and claim/evidence references;
- hard policy predicates and their result;
- residual leakage/failure matrix;
- exact workload/source/configuration/receipt digests;
- request-to-accepted-result latency and energy;
- all rejects, failures, deferred/indeterminate outcomes and retry costs;
- provider-side reserved capacity, local QoS impact and owner withdrawal response.

An arm with better raw latency but lower privacy, weaker correctness or an unauthorized data disclosure belongs at a different point on the frontier; it is not a universal improvement.

## 8. Claim boundary

This document defines a proposed compositional contract and benchmark interpretation. It does not assert an implemented guarantee vector, cryptographic proof, private ANN service, correct scheduler, or production qualification.
