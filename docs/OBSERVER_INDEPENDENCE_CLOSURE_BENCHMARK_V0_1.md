# Observer Independence Closure Benchmark v0.1

**Status:** synthetic fixture contract only. This supplement extends the existing Agentic Endpoint Assurance Benchmark v1 and reuses its oracle-fidelity roles/channels and Mycelix common-cause ownership. It does not create a new failure-domain graph, trust registry, evidence store, witness service or runner.

## Paradigm shift: independence is relational

A process count is not an independence proof. Two observers may run in different processes yet trust the same event stream, signing key, credential root, clock, storage backend or control plane. If that common component is compromised or unavailable, both observations can fail together.

OIC models independence as a claim-specific set of disjoint failure domains. The v0.1 critical-effect-truth profile requires separation across:
- process/runtime;
- source event stream;
- signing root;
- credential root;
- clock authority;
- storage backend;
- control plane.

Operator and provider are recorded separately but are not required to differ in this specific v0.1 profile. A stricter claim profile may require additional domains; profile changes must be explicit, versioned and requalified.

A witness with missing or unknown required-domain identity is not assumed unique. A witness sharing any required domain with the subject under observation is subject-correlated. Two otherwise distinct witnesses sharing a required domain are pairwise correlated.

## Quorum semantics

The checker computes the maximum set of witnesses that are:
1. individually disjoint from the subject across every required domain; and
2. pairwise disjoint from one another across every required domain.

Only a maximum independent set meeting the configured quorum can qualify the observation. All maximum independent sets must agree on the same known observation. Disagreement is indeterminate; a simple majority never overrides a conflicting independent witness. If the maximum independent set is below quorum, missing domain evidence makes the result indeterminate; known common-cause correlation makes it unqualified.

## Independent measurements (no aggregate score)

- witness count and known/unknown domain coverage;
- subject-correlated witness count;
- pairwise correlated witness count and exact shared dimensions;
- maximum independent witness-set size and all maximum sets;
- required quorum;
- agreement/disagreement between independent observations;
- final disposition: qualified effect, qualified no-effect, unqualified or indeterminate.

The profile explicitly lists the domains it requires. Sharing a domain outside that profile is recorded but does not silently disqualify a claim; the benchmark must not smuggle in universal assumptions about operator/provider independence.

## Frozen synthetic coverage

16 scenarios: 4 positive controls, 9 negative controls and 3 indeterminate controls. The suite covers:
- different processes sharing event streams and signing roots;
- shared signing root, credential root, clock authority, storage backend and control plane;
- missing critical-domain identity;
- disagreement between truly independent witnesses;
- qualified no-effect and effect quorums;
- witness correlation with the subject's own event stream;
- maximum independent subset among a correlated group;
- unknown witness domains alongside a separate sufficient quorum;
- weakened claim profile and duplicate witness identity.

Files:
- `benchmarks/agentic-endpoint-security-v1/observer-independence-closure.schema.json`
- `benchmarks/agentic-endpoint-security-v1/observer-independence-closure.fixtures.json`
- `tools/agentic-endpoint-security-v1/check_observer_independence_closure.py`
- `tools/agentic-endpoint-security-v1/test_observer_independence_closure.py`

The standard-library checker reports `STRUCTURE_CHECKED`; the schema is Draft 2020-12 for external validation. A checker pass is not a live common-cause fault-injection campaign.

## Ownership and comparison

- Platform oracle fidelity v1 owns observer roles, side-effect channels and endpoint fault-control contracts.
- Mycelix #5762 owns the common-cause/dependency cut-set model consumed by future runtime adapters.
- Symthaea ASSURE/WALP owns claim lifecycle/currentness and qualification evidence.
- Resource/participant owners retain final effect truth and admission authority.

HCP-style independent gateways, AgentCap-style capability runtimes and external observers are comparators, not proof of failure-domain separation. A matched test must inventory their event sources, signing roots, credentials, clocks, storage, control planes, operators and providers. Two observers with shared critical infrastructure must not count as two independent votes.

## Claim ceiling

This v0.1 model evaluates synthetic domain identifiers. It does not discover hidden shared infrastructure, inspect real key hierarchies, prove clock independence, validate provider/operator contracts, verify cryptographic signatures or inject common-mode failures into deployed systems. Runtime promotion requires owner-native dependency evidence, independently verified domain identities, actual common-cause fault injection, exact-head CI and independent audit.
