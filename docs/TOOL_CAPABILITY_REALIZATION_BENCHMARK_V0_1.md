# Tool Capability Realization Benchmark v0.1

**Status:** synthetic capability-surface contract only. No compiler-grade static analysis, binary inspection or live runtime instrumentation is claimed.

## Purpose

This benchmark operationalizes [WEB4-TOOL-CAPABILITY-REALIZATION-001 #516](https://github.com/Luminous-Dynamics/luminous-platform/issues/516).

It compares three distinct surfaces for an exact tool artifact:
1. **Declared effects:** what the tool description/schema/manifest claims.
2. **Implementation-derived potential:** what the exact source/build/dependency/configuration may reach, including reachable and conditional effect paths.
3. **Observed effects:** what an independently qualified resource-side observer actually saw.

A declaration is not an enforcement boundary. A static path is not proof that the effect occurred. An agent/observer's self-report is not independent effect truth.

## Owner boundaries

- CAPC v0.1 owns canonical resource/effect identities; the fixture effect tuple is the CAPC tuple.
- AUTH-CORE-002 / IFL own source, action and parameter-origin lineage.
- Mycelix owns grant authority and operational dependency semantics.
- Symthaea ASSURE/WALP owns claim lifecycle/currentness.
- Participant/resource owners own final admission and effect truth.
- This Platform supplement owns the comparison contract and synthetic controls.

No new resource registry, capability engine, policy engine, sandbox, effect oracle or benchmark runner is introduced.

## Exact artifact binding

Each case binds tool name and human version label to immutable source commit, source-tree and dependency-lock digests, build provenance, produced binary digest, runtime configuration, tool-manifest/description digest and static-analysis identity.

A friendly version label is not artifact identity. Two builds with the same label must remain distinct if source or binary identity differs. Missing source/build/binary correspondence makes the exact implementation claim indeterminate.

Synthetic SHA-256-shaped values exercise canonical formatting and binding semantics; they are not hashes of real artifacts.

## Set semantics

Let (D) be the set of CAPC canonical effects declared by the tool, (P) the set of canonical effects reported as reachable or conditional by analysis of the exact implementation, and (O) the set of canonical effects independently observed at the resource boundary.

Report separately:
- potential undocumented surface: (|P \setminus D|);
- observed undocumented effects: (|O \setminus D|);
- observed effects missing from the static inventory: (|O \setminus P|);
- declared but not represented in the implementation surface: (|D \setminus P|);
- unresolved implementation paths;
- artifact-binding completeness and observer qualification.

Interpretation:
- (P \setminus D) is potential exposure, **not an observed breach**.
- (O \setminus D), with a complete independent observer and exact artifact binding, is an observed undeclared effect.
- (O \setminus P) is an implementation-analysis closure failure.
- (D \setminus P) is an over-declared surface / potential false-refusal risk, not proof that the runtime is safe.
- Unknown source/build/runtime dependencies or unqualified observation remain indeterminate.

The checker consumes normalized facts; it does not perform static analysis or observe an operating system.

## Frozen synthetic panel

12 scenarios cover exact agreement and route aliases; dormant undocumented file-write paths; independently observed undeclared writes; over-declared delete capabilities; unresolved dynamic plugin loading; conditional FFI/process-spawn capabilities; observed network effects missing from static inventory; unqualified observer claims; same friendly version label with distinct source/build identities; missing binary provenance; and opaque remote extension paths.

Distribution: three positive, five negative and four indeterminate controls. No aggregate score.

## Proposed gates (targets, not results)

- zero observed effects outside the declared and admitted effect set in critical controls;
- zero unknown implementation/dependency paths promoted to complete or safe;
- 100% exact source/build/runtime identity for scored episodes;
- 100% expected deterministic fixture outcomes;
- zero effect-identity collisions or alias fragmentation after CAPC normalization;
- zero untrusted observer assertions promoted to observed effect truth;
- report static potential, observed effects, analysis gaps, over-declaration and benign utility separately.

A passing checker qualifies only synthetic semantics. Static-analysis promotion needs pinned analyzers/builds and independent ground-truth adjudication. Runtime promotion requires a disposable target, OS/resource-side observation and exact build correspondence.
