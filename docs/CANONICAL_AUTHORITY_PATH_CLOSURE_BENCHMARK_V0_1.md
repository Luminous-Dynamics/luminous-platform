# Canonical Authority Path Closure Benchmark v0.1

**Status:** synthetic contract only; no real resource-owner resolver, route inventory, MCP/REST/SDK connector or effect observer has been exercised. CAPC extends the existing TWEP/Agentic Endpoint Assurance Benchmark v1 in PR #141. It introduces no second runner, registry, policy engine, dependency graph or effect observer.

## Why CAPC follows TWEP

TWEP keeps task-required, granted, available, exercised and effected authority separate over a trajectory. Its v0.1 authority atoms are exact actor/capability pairs; a path label is not proof that two routes reach the same protected resource or effect.

CAPC adds owner-issued canonical resource/effect identity and independently inventoried route closure. The owner remains the source of canonical resource identity. Route adapters report concrete paths and credential lineage. The independent observer reports actual resource-side effects. CAPC only qualifies the cross-layer mapping and measures gaps; it does not authorize actions.

## Canonical effect tuple

Every owner-approved or owner-resolved effect is bound to:
- owner identity, tenant/domain and resource identity;
- semantic operation and effect class (read/write/create/update/delete/send/transfer/execute/delegate);
- principal, workload and session identity;
- argument/value-constraint commitment;
- resource version precondition and policy epoch;
- resolved service identity and credential lineage;
- owner-issued canonical effect ID.

An MCP method name, REST URL, SDK method, model-generated description or caller-supplied alias is not a canonical effect identity. v0.1 treats the argument commitment as an opaque owner-supplied digest; it does not compute or verify a canonical serialization hash.

## Path inventory and mapping

The denominator comes from an independent route inventory, not from the agent's advertised tool list or its own log. Each inventory row identifies a route, route kind, credential reference, whether it is declared in the effective authority manifest, and whether the path is reachable, unreachable or unknown.

Every known path needs exactly one owner-attested mapping. A path counts as closed only when the mapping is resolved, owner-attested, tied to the canonical resource owner, carries a reference/digest, and resolves to a consistent canonical effect tuple. Missing, ambiguous, unattested or unknown paths remain outside the closure numerator and inside the visible denominator.

## Two dual canonicalization failures

1. **Identity collision:** one canonical effect ID is assigned to different semantic tuples (e.g. two tenants or resources). All conflicting path mappings are excluded from closure.
2. **Identity fragmentation:** identical semantic tuples are assigned multiple canonical IDs across routes. These paths are also excluded from closure.

Equivalent MCP/REST/SDK routes should share one ID and one exact semantic tuple. Identical route labels resolving to different tenants/resources must remain distinct. Read and write operations on the same resource must remain distinct. A path mapping must also agree with the effect observed at the resource boundary; an event cannot borrow authorization from another approved effect just because that other effect is also in the approved set.

## Independent measurements (no aggregate score)

- path-closure numerator / independent-inventory denominator;
- unresolved, ambiguous and unattested path count;
- number and size of owner-equivalent route groups;
- canonical identity collisions and semantic fragmentation;
- reachable shadow paths outside declared authority inventory;
- shadow-path protected effects;
- resource/tenant/operation/effect-class substitutions;
- credential/principal/workload/session lineage substitutions;
- argument/value-constraint widening;
- observed-effect identity mismatches between the route mapping and resource-side observation;
- unauthorized attempts, unauthorized admissions and independently verified unauthorized effects;
- false refusals on exact, owner-attested, approved paths;
- observer completeness/independence and indeterminate cases.

A path-closure percentage without the independent denominator and unresolved-path count is not a valid claim. A zero unauthorized-effect count is not a security claim without independent, complete resource-side observation and adequate episode counts.

## Synthetic controls

15 cases: 5 positive, 6 negative and 4 indeterminate. They cover MCP/REST/SDK equivalence, same route label with different tenants, read/delete and read/write separation, argument-scope widening, dynamic unresolved paths, shadow path blocked before effect, credential lineage substitution, canonical ID collision, unattested owner mapping, same-domain observers, false refusals, ambiguous resolution, identity fragmentation, and an observed effect that differs from the exact path mapping even though both effects are in the approved set.

Files:
- `benchmarks/agentic-endpoint-security-v1/authority-path-closure.schema.json`
- `benchmarks/agentic-endpoint-security-v1/authority-path-closure.fixtures.json`
- `tools/agentic-endpoint-security-v1/check_authority_path_closure.py`
- `tools/agentic-endpoint-security-v1/test_authority_path_closure.py`

The standard-library checker labels its schema check as structural; full Draft 2020-12 validation is available to external validators. The checker and mutation tests are added to the existing read-only workflow.

## Comparator interpretation

HCP v2 explicitly includes canonical resource resolution among eight execution-layer invariants. It reports 18/18 modeled violations blocked and 25/25 benign workflows completed for both its stateful runtime and matched independent gateway ACL. On 24 held-out AgentDojo tasks, it reports attacker-call denial but also 40 non-attacker calls denied under strict reference-derived grants; its third-party server study finds per-call grants alone insufficient to prevent secret-to-authorized-sink argument flow. These results are external paper results, not Luminous reproductions.

AgentCap already provides signed, short-lived, budget-bounded capabilities and attenuating parent/child chains. CAPC does not claim novelty for those controls. Its question is whether route/resource resolution is closed and independently evidenced across every reachable path, and whether actual resource effects match the exact path-specific canonical tuple.

NIST's 27 October 2026 agent identity design tenets emphasize distinct cryptographically verifiable agent identity, delegation attenuation, least/zero standing privilege, and short-lived audience-specific runtime-verified authorization. CAPC is not a NIST profile or conformance claim; it supplies proposed measurements for resource/effect identity and route closure.

## Claim ceiling

CAPC v0.1 does not verify owner signatures, prove that the independent inventory is complete, discover every hidden network route, canonicalize real resource paths, attest cryptographic digest contents, validate authorization policy, or observe physical/OT effects. The boolean owner-attested and observer flags are synthetic facts. Runtime claims require actual owner-side resolvers, independently sourced inventory, path-level credential provenance, resource-side postcondition observers, matched comparator implementations, exact-head CI and independent review.
