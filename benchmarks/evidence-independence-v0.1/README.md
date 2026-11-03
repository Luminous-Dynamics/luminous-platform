# Evidence Independence Profile v0.1

This directory defines a first, deliberately small profile for describing which components and control domains support an independence claim.

It is an **assurance input**, not an attestation, certificate, trust root, or truth oracle. JSON Schema validation establishes only that a declaration has the expected shape. It does not authenticate a component, prove that two administrators are distinct, establish that a sensor observed reality, or prove that a dependency inventory is complete.

## Why a vector, not a boolean

A process ID is one execution fact. It does not establish independent control over the host, administrator, credentials, persistent state, network path, time source, build pipeline, or observation channel.

The profile therefore records eight dimensions separately:

| Dimension | What must be distinguished |
| --- | --- |
| `execution` | Process, OS identity, container/VM, host, kernel, supervisor |
| `administration` | Operator, administrator, deployment and recovery authority |
| `authority` | Grant issuer, credential issuer, key controller and rotation authority |
| `state` | Database, storage, replication, backup and restore control |
| `communication` | Network path, proxy, gateway, DNS, IPC and telemetry collector |
| `time` | Clock, synchronization authority, monotonic clock and session identity |
| `construction` | Source revision, build pipeline, toolchain and artifact provenance |
| `observation` | Sensor/probe, observer implementation, acquisition path and operator |

Each dimension has one of three states:

- `known`: one or more pseudonymous domain IDs and evidence references are declared;
- `unknown`: the relevant control domain is not established;
- `not_applicable`: the dimension genuinely does not apply to this component.

An unknown required dimension makes the relevant claim indeterminate. A not-applicable required dimension is also indeterminate; it must not be treated as independence.

## Profile semantics

A claim identifies one evidence-producing component, the components from which it must be independent, and the dimensions required by that claim. Any shared domain ID in a required dimension fails the claim. A common dependency discovered transitively must also be treated as shared. The initial schema does not define exceptions for shared domains; any future exception must be explicit, narrowly scoped, evidence-backed, and separately reviewed.

The auditor must validate component references, duplicate IDs, dependency cycles, domain intersections and transitive dependencies. It must not trust an assessor-supplied pass flag. It must derive its result from the input profile and independently verified evidence references.

The eight dimensions are not a universal checklist for every claim. For example, a reproducible analytical calculation may require independent construction and execution provenance; a service-health claim additionally requires independent observation and communication paths. The claim-specific requirement is part of the threat model, not a score.

## ITOPS process-smoke fixture

`itops-process-smoke.profile.json` deliberately models the topology in Luminous Platform PR #217:

- runner, participant reference process and observer reference process have distinct process identities;
- all three share a host, OS user, supervisor, administrator, synthetic authority, clock, build artifact and IPC path;
- participant outcomes and observer health samples are supplied by the runner rather than measured from a real participant effect sink or running service.

The expected result is therefore **process-separated prototype only**. It must fail a claim requiring separately administered participant or observer evidence. The fixture is a negative control, not proof of any deployed control-domain fact.

## How other programmes can reuse it

- **ITOPS:** bind the profile digest to the episode/observer receipt; the agent must not author or upgrade the profile.
- **Symtropy:** declare solver, scenario generator, reference engine, observation adapter, build and host dependencies. Solver disagreement remains evidence to investigate, not physical truth.
- **Mycelix:** declare verifier, challenge issuer, authoritative registry, key controller, storage/failover and transparency domains. A signature or challenge equality is not a substitute for consume-once semantics.

The profile does not replace ITOPS source-evidence receipts, Symtropy's SCITT-aligned evidence work, Mycelix's credential/signature profiles, or their domain-specific verifiers. It introduces no new signature format, key hierarchy, policy engine, ledger, or transparency service.

## Verification boundary

This first slice contains the schema and one known-weak fixture. It does not yet contain the Rust graph auditor, evidence-reference resolver, authenticated domain discovery, platform isolation probes, or production deployment profiles. No claim of independence or security is established by this directory alone.

Next implementation should add:
1. strict schema execution and semantic validation;
2. transitive dependency and common-domain analysis;
3. per-dimension `pass`, `fail`, and `indeterminate` results with a claim ceiling;
4. mutations for shared host/user/supervisor, credentials, database, clock, network, collector, key controller and build authority;
5. offline reproducible reports with exact input/schema/tool digests.
