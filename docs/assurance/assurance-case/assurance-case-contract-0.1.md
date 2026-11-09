# Executable assurance case contract 0.1

**Status:** initial executable reference; public synthetic conformance only; not a certification scheme  
**Parent:** [ASSURANCE-CASE-001 / Platform #345](https://github.com/Luminous-Dynamics/luminous-platform/issues/345)  
**Benchmark:** [OMSI-EVAL-001 / Platform #325](https://github.com/Luminous-Dynamics/luminous-platform/issues/325)

## Industry context

ISO/IEC/IEEE 15026-2:2022 specifies minimum requirements for assurance-case structure and terminology. It does not require a particular graph or physical representation and does not establish the quality of case content. Goal Structuring Notation (GSN) is widely used to present safety and assurance arguments. NASA's AdvoCATE research demonstrates automated construction, assembly, pattern instantiation, formal-method integration and argument verification.

The proposed contribution is not another diagram editor. It is an executable evidence and defeater semantics layer that consumes evidence owned by independent systems and returns a scoped, reconstructable argument assessment.

## Contract model

An assurance case contains:
- **Claims** with decision scope and explicit required evidence, subclaims and argument nodes.
- **Argument nodes** with rationale and evidence support.
- **Context nodes** that bind system and operating scope.
- **Assumptions** with supported, bounded, open or refuted state.
- **Evidence** with exact artifact digest, source, provenance, evidence class, scope, status, observation time, bounded validity interval and optional dependency assessment.
- **Defeaters** with applicability, criticality, description and raised time.
- **Rebuttals** that target one exact defeater and cite evidence observed no earlier than that defeater was raised.

The edge vocabulary is constrained to `decomposes_into`, `supports`, `contextualizes`, `assumes`, `defeats`, `rebuts` and `depends_on`. Invalid endpoint types and unsupported cycles are rejected.

## Decision semantics

- `SUPPORTED_FOR_DECLARED_SCOPE`: required support is present and valid at the case's `as_of` time; no critical open defeater or unresolved assumption blocks the argument.
- `DEFEATED`: a critical applicable defeater remains open, a critical assumption is refuted, or required independence evidence reports a shared dependency.
- `INDETERMINATE`: evidence is missing, stale, revoked, conflicted, out of scope, temporally invalid, dependency-unknown, or a noncritical defeater/open assumption remains unresolved.
- `INVALID_ARGUMENT_GRAPH`: invalid structure, dangling references, wrong edge types, unbound assumption/rebuttal evidence, unsupported cycles or malformed contract.

A rebuttal never deletes the defeater. It closes that defeater only for the current evaluation when its exact target is referenced and all cited evidence is current, in scope, temporally valid and observed no earlier than the defeater's raised time.

## Dependency semantics

The assurance evaluator consumes a typed dependency assessment; it does not recompute the dependency graph.
- `SHARED_DEPENDENCY` defeats a claim requiring independent corroboration.
- `UNKNOWN_DEPENDENCY` is indeterminate.
- `NO_DECLARED_COMMON_DOMAIN` may support only that scoped, complete-inventory proposition.
- `NO_DECLARED_COMMON_DOMAIN` cannot prove physical or statistical independence.

Mycelix remains the owner of dependency closure and its independent verifier. Provenance and sealed-evaluation remain the owners of byte integrity and custody evidence.

## Conformance and adversarial properties

The current public software suite contains 26 Rust tests plus a 25-ID frozen registry. It covers positive support, missing/stale/out-of-scope evidence, assumption and defeater propagation, fresh versus stale rebuttals, shared/unknown dependencies, physical-independence overclaim, inventory incompleteness, dangling/wrong-type edges, cycles, argument-only support, malformed references and exact-byte report binding.

The graph validator builds incoming/outgoing edge indexes once instead of repeatedly scanning the entire edge set per node. The benchmark programme should still measure p50/p95/p99 latency and peak memory; no performance result is claimed by the implementation alone.

Passing this suite is not a blind evaluation. Blind generalization requires evaluator-held truth, access isolation, post-lock reveal and external review through the sealed-evaluation protocol.

## Proposed measures

Initial internal targets:
- 100% exact outcome and reason-code agreement across the frozen case registry.
- Zero supported outcomes with open critical defeaters or unresolved critical assumptions.
- Zero use of stale, revoked, conflicted, missing, out-of-scope or temporally invalid evidence to satisfy required obligations.
- Zero promotion of no-declared-common-domain into physical/statistical independence.
- 100% exact input-byte digest binding.
- 100% exact rebuttal target and freshness binding.
- Deterministic report reconstruction.
- Report p50/p95/p99 validation latency and peak memory at 10, 100, 1,000 and 10,000 nodes and several evidence/edge densities.

These are proposed software conformance targets, not standards or field-risk guarantees.

## Claim ceiling

The v0.1 package validates graph structure and declared evidence obligations for a synthetic contract. It does not prove evidence is physically true, authenticate source signatures, establish organizational independence, prove the premises of an argument, or provide safety/regulatory certification.
