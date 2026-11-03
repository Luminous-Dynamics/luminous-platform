

## Operational dependency evidence adapter

The [operational dependency supplement](assurance-interop/README.md) records the concrete ITOPS dependency topology (host/kernel, OS identity, supervisor, authority, participant state/effect sink, replication/failover, IPC/network, time, service probes and telemetry) without creating a competing generic independence profile.

It maps to Mycelix's canonical QUAL-001E / #1371 verifier-correlation profile, ASSURE-PORTFOLIO-CUT-001 / #5695 (implemented in #5696), and LEX-NET-030 / #1243 evidence-dependency graph. The machine-readable contract is [operational-dependency-supplement.schema.json](assurance-interop/operational-dependency-supplement.schema.json), with a weak-topology negative fixture at [INC-002 operational dependencies](assurance-interop/fixtures/inc-002-operational-dependencies.json).

The fixture documents the present #217 process smoke: separate process IDs but shared host, OS identity, supervisor, administration, synthetic authority, IPC, clock and build artifact. No agent, real participant effect sink, running service probe or independent runtime monitor is exercised. The fixture is **source-derived topology evidence**, not a runtime attestation, and its expected claim ceiling is process-separated prototype only. Canonical Mycelix profile, policy and receipt digests remain explicitly unbound until supplied by their owning verifiers.
