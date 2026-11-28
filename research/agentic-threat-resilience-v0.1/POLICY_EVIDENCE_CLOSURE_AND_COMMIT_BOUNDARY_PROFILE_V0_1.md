# Policy Evidence Closure and Commit-Boundary Profile v0.1

**Status:** Synthetic conformance profile. No production PEP, source resolver, cryptographic verifier or external observer is implemented.

## Problem

A decision can be internally consistent yet wrong because a required fact was omitted. Recent tau2-bench research found that a gate's evidence digest dropped order status and customer authentication; the resulting gates did not significantly reduce premature commitments and could over-refuse valid actions. It also found a large disagreement between LLM-judge, native-state and commit-boundary evaluators. See [Im (10 Nov 2026)](https://arxiv.org/abs/2611.14581).

## Design shift: authorization decisions require evidence closure

A policy's dependency set must be explicit and versioned. Every evaluated predicate must declare all facts it consumes. A decision is usable only when:
- the predicate dependency set exactly matches the policy's declared dependency manifest;
- every required fact is supplied exactly once;
- each fact comes from an accepted source, has source-authenticated trust, a source version, an observation time and an active status;
- each fact is within its freshness bound;
- actor, principal, tenant and resource match the resolved authority context;
- the decision binds the same opaque Act-Bound request digest and evidence-bundle identity later presented to the participant.

Missing, duplicate-conflicting, stale, candidate-only, revoked or unresolved facts yield **indeterminate**, not allow. A preflight allow is not a commit permit: the participant revalidates dynamic dependencies, resource version, authority epoch and exact action/evidence identities at the commit boundary.

## Separate correctness axes

1. Task correctness: did the agent complete the valid task or correctly refuse?
2. Authorization correctness: was the exact actor/tenant/resource/operation permitted?
3. Commitment correctness: did the resource mutate if and only if the task and policy required?
4. Evidence completeness: did the decision include every required source-qualified dependency?
5. Effect attribution: can the actual resource effect be independently tied to the authorized action?

An LLM judge is auxiliary dialogue annotation. It cannot override an unauthorized participant commit or incomplete evidence closure.

## Synthetic corpus

The 20 cases PEV-001..020 cover complete allowed writes, missing authentication/order facts, stale data, candidate-supplied claims, hidden dependencies, prohibited commits, action/evidence substitution, actor/tenant substitution, authority/resource drift, under-action, unknown outcomes, unsafe retry, safe refusal, judge disagreement, effect without participant receipt, conflicting source facts and read-only utility.

The reference checker also runs 16 one-field mutations. These tests verify the synthetic contract only; they do not perform cryptographic hashing, signature verification, source authentication or real resource observation.

Files: `schemas/policy-evidence-closure-corpus-v0.1.schema.json`, `scenarios/policy-evidence-closure-corpus-v0.1.json`, `tools/validate_policy_evidence_closure.py`, and `.github/workflows/atr-policy-evidence-closure.yml`.

## Integration boundaries

- Reuse the existing Act-Bound canonical action digest; no second canonicalizer or digest algorithm.
- Reuse the existing identity/delegation and Mycelix/Xenia authority resolvers; allowed actor lists are resolved inputs, not authority minted by this profile.
- Reuse #540's participant and independent observer as the real effect oracle.
- Reuse #546 for request-to-digest binding and #553 for the policy evidence closure work.
- Keep the policy dependency manifest versioned with the policy itself and revalidate mutable inputs at participant commit.

## Claim ceiling

The fixtures contain synthetic booleans/values representing upstream verifier outputs. A passing schema/reference workflow is not proof of production authorization, completeness of any deployed policy, commit-time atomicity or independent effect truth.

## References

- Im, *When Evaluators of AI Agents Disagree*, 10 Nov 2026: https://arxiv.org/abs/2611.14581
- ReliabilityBench: https://arxiv.org/abs/2601.06112
- tau-bench: https://arxiv.org/abs/2406.12045
- Platform #540, #546, #553.
