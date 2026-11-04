# ITOPS operational dependency supplement v0.1

**Purpose:** record the operational dependency topology that an ITOPS episode needs to expose to the existing Mycelix assurance stack.

This is not a second generic independence profile. Canonical ownership remains:

- [QUAL-001E / Mycelix #1371](https://github.com/Luminous-Dynamics/mycelix/issues/1371): verifier corroboration lineage and 15 fault-domain dimensions.
- [ASSURE-PORTFOLIO-CUT-001 / Mycelix #5695](https://github.com/Luminous-Dynamics/mycelix/issues/5695), implemented as [PR #5696](https://github.com/Luminous-Dynamics/mycelix/pull/5696): profile completeness, pairwise separation obligations and bounded common-cause cut sets.
- [LEX-NET-030 / Mycelix #1243](https://github.com/Luminous-Dynamics/mycelix/issues/1243): dependency lineage between evidence sources, issuers, trust roots, registries, operators, data sources, sensors and derived artifacts.
- [ITOPS #227](https://github.com/Luminous-Dynamics/luminous-platform/issues/227): operational adapter and this supplement.
- [ITOPS runner #213](https://github.com/Luminous-Dynamics/luminous-platform/issues/213), [process-smoke PR #217](https://github.com/Luminous-Dynamics/luminous-platform/pull/217), and [broker boundary #168](https://github.com/Luminous-Dynamics/luminous-platform/issues/168).

## Why this sidecar exists

The Mycelix 15-domain model defines broad, generic fault-domain identities. ITOPS still needs to retain multiple concrete operational dependencies inside those domains. For example, a service observer can depend on a specific host/kernel, process supervisor, network path, DNS/proxy, source probe, telemetry collector and time source. A participant ledger may have a primary database, replication/failover control, backup and restore authority. These are not interchangeable identities and must not be collapsed into one opaque “execution domain” value.

The supplement records:
- episode components and their execution status;
- multiple operational dependencies per component and dependency kind;
- explicit edges linking components to those dependencies;
- claim-to-source bindings for participant effects and service health;
- monitor-coverage status and event-loss information;
- references to canonical Mycelix profile/policy/receipt commitments.

It does **not** calculate a universal independence score or issue a generic “independent” verdict. Consumers must adapt these entries into QUAL-001E/ASSURE-PORTFOLIO-CUT and LEX-NET-030 with exact policy, profile, subject and receipt digests. No digest is fabricated when the adapter is not bound.

## Data semantics

Dependency status is deliberately multi-valued:
- `observed_in_fixture`: the exact dependency identity is represented in the source fixture/protocol evidence; this is not proof of real-world administration or deployment ownership.
- `declared_shared`: source evidence explicitly indicates that the parties share this dependency.
- `not_exercised`: the current experiment did not instantiate or measure this dependency.
- `unknown`: the dependency may exist but its identity is not known.
- `not_applicable`: the dependency is outside the declared scenario, with the reason retained.

A missing field is not equivalent to `not_applicable`. A process ID is not a host identity. A signed observer receipt is not a probe measurement. Distinct endpoint names do not establish independent acquisition paths.

The dependency graph in v0.1 is a one-hop component-to-operational-dependency sidecar. When a dependency itself has dependencies (for example, a database failover service depends on its own DNS, host and operator domains), encode the additional components as explicit dependency nodes in a later schema version or bind to the full Mycelix LEX-NET-030 graph. Do not infer transitive closure from friendly labels or values that are not authenticated.

## #217 fixture interpretation

The fixture `fixtures/inc-002-operational-dependencies.json` describes the current process smoke:

- runner, participant reference process and observer reference process have distinct process identity labels;
- they share a host/kernel, OS identity, supervisor, administration domain, synthetic authority, IPC path, clock and build artifact;
- participant outcomes are harness-supplied synthetic labels, not readback from a participant-owned business-effect sink;
- observer health values are harness-supplied synthetic samples, not a probe of a running service;
- no agent-under-test is executed; no production broker, database, DNS path, telemetry collector, runtime access monitor, or independent reset authority is exercised.

Expected claim ceiling: **process-separated prototype only**. It must not be described as independently observed participant effect or service health.

## Binding rules

Before an episode may consume this supplement as assurance evidence:
1. bind exact episode, scenario variant, corpus and SUT identity;
2. bind the sidecar to a canonical QUAL-001E profile and external ASSURE-PORTFOLIO-CUT policy by exact digests;
3. bind source/derivation edges to LEX-NET-030 and exact evidence receipts;
4. verify component/dependency/edge references and reject dangling nodes, duplicate IDs, conflicting identities or unsupported statuses;
5. verify monitor continuity and treat any gap, loss, missing monitor, or unknown source as indeterminate;
6. preserve every shared and not-exercised dependency in the resulting claim view.

The JSON Schema is executed by the existing `itops-qualification-core` Draft 2020-12 validator. The public `audit_operational_dependency_supplement` function then checks component/dependency/edge references, duplicate edges, role-to-claim bindings, recomputes direct shared dependency identities using `(canonical_fault_domain, identity_label)` rather than node IDs, checks required sink/probe exercise status, and verifies that declared claim findings match the recomputed topology. It also compares each oracle to the runner and the agent-under-test; an absent agent is explicitly reported as `compared_component_not_exercised`. A monitor cannot be marked complete unless its component is a process-observed runtime access monitor. Its result is still not an independence verdict. It does not authenticate component identities, verify domain membership, prove dependency inventory completeness, or replace the Mycelix semantic profile checker. It deliberately analyzes only direct ITOPS component-to-dependency edges; transitive evidence-source derivation remains owned by LEX-NET-030.

## Research and benchmark direction

The next operational campaign should include matched fault injections for:
- shared host/kernel/supervisor versus separately hosted observer;
- shared and distinct participant state/replication/failover domains;
- lost response before/after participant commit;
- retry and duplicate delivery while prior outcome is unknown;
- shared versus distinct DNS/proxy/network/IPC paths;
- clock skew, rollback and time-source loss;
- observer source substitution, stale data, telemetry collector loss and sequence gaps;
- monitor crash, reset, reboot, and incomplete event coverage;
- backup/restore and failover recovery with digest tombstones preserved.

Report each dependency class, common-cause cut and resulting claim ceiling. Do not publish a single independence score.
