# Tiered sandbox assurance and resource telemetry v0.1

## Purpose

This extension makes the actual isolation boundary a first-class property of every OpsBench run. A run is not simply "sandboxed": it declares and independently attests one of four tiers. Results are stratified by tier; a stronger isolation tier does not earn extra task-success points.

The implementation reuses the sealed manifest, freeze receipt, runtime observer, and source-evidence architecture. It does not create a new policy engine, evidence journal or signature system.

## Tier definitions and required evidence

### Tier 0 — Research

No isolation claim. Suitable for public development, local experiments and deterministic evaluator tests. The profile must explicitly identify itself as unisolated. Logical/resource measurement is required; physical energy may be explicitly unavailable. A Tier 0 run must never be presented as process/container/microVM qualified.

### Tier 1 — Process

The SUT runs as a separated process under a measured host boundary. Required evidence:
- seccomp-BPF policy digest;
- namespace profile digest;
- cgroups v2 resource-control profile digest;
- host kernel, architecture and hardware identity;
- external Supervisor with hardware-protected signing key class;
- complete logical and resource telemetry.

Accepted mechanism identifiers in v0.1: seccomp_process or gvisor_process. A string naming a tool is not proof; the Supervisor receipt must bind the deployed profile and adapter identity.

### Tier 2 — Container

The SUT runs in a content-addressed container with controlled mounts, network namespace and budgets. Required evidence:
- content-addressed image digest;
- mount policy and network namespace digests;
- cgroups v2 resource-control profile;
- tool sub-sandbox digest (for example a WASI/Wasmtime boundary);
- host kernel, architecture and hardware identity;
- hardware-protected external Supervisor.

Accepted mechanism identifiers: container_runtime or gvisor_container.

### Tier 3 — MicroVM

The SUT runs behind an independently identified guest-kernel boundary. Required evidence:
- Firecracker, Cloud Hypervisor or crosvm runtime identity;
- hypervisor and guest-kernel digests;
- guest mount/network namespace policy;
- cgroup resource profile and dedicated CPU-set digest;
- tool sub-sandbox digest;
- host hardware identity;
- hardware-protected Supervisor;
- complete physical telemetry, including calibrated CPU package energy.

If an accelerator is assigned, its allocation digest and measured GPU energy are required. Dedicated CPU/accelerator claims must be corroborated by host-side telemetry, not merely guest-reported metadata.

## Three-way separation

- **SUT:** candidate or baseline agent and its model/scaffold; cannot access Supervisor keys or evaluator-only oracle data.
- **Harness:** task inputs, tools, runtime and agent scaffold; content-addressed and itself measured.
- **Supervisor:** external to SUT and Harness; controls measurement, observes host-side facts, signs receipts with a key inaccessible to the SUT/Harness, and publishes through the existing typed source-evidence path.

The frozen manifest pins sandbox tier/profile digest, Supervisor ID/source/control domain/adapter digest and key-protection class. The freeze receipt repeats those fields. Each per-trial Supervisor receipt repeats the frozen execution-environment and resource-budget digests so the measured host/runtime and budget are explicitly joined to the trial. Supervisor control domain must differ from candidate, baseline, evaluator, freeze authority, runtime observer and scenario/effect evidence domains.

## Telemetry contract

Each (trial, arm) gets exactly one supervisor attestation. It binds:
- manifest, scenario and candidate/baseline identity;
- the exact frozen execution-environment and resource-budget digests;
- tier and profile digest;
- Supervisor identity and hardware-key class;
- runtime instance and unique writable namespace;
- host OS, architecture, kernel and hardware identity;
- tier-specific runtime boundary digests;
- measurement interval, clock uncertainty and contiguous receipt sequence.

Telemetry is separated into three categories.

### Logical

Session identity, harness identity, transcript commitment, input/seed commitment, tool-call count and input/output tokens. Full trace replay is held outside the public oracle boundary; public reports expose commitments and sanitized details, not private task answers or customer data.

### Resource

Wall-clock, CPU time, peak memory, disk/network bytes, tool calls, tokens, retries, and accelerator utilization/memory where supported. Report per arm, per trial and per independently verified outcome. Never compare cost without including failed attempts and retries.

### Physical

CPU package/GPU energy, device identity, sampling interval, calibration profile, measurement source and uncertainty. Unsupported sensors are explicitly marked unavailable; the value is not zero. A Tier 3 physical-energy claim requires a calibrated CPU package energy reading that covers the entire trial interval. GPU energy is additionally required if an accelerator was allocated.

Energy is an efficiency metric, not a quality or safety proxy. Do not report joules/verified recovery when the energy measurement or the verified-recovery denominator is incomplete.

## Timing, sequence and hardware constraints

- Supervisor measurement starts before the first activity in the corresponding arm and ends after its final terminal event, including a skip disposition.
- Clock uncertainty is bounded; both ends are checked conservatively.
- Exactly one receipt per planned trial × arm is required.
- Supervisor sequence numbers start at one, are contiguous and strictly ordered by issued time within the run. Cross-run replay/anti-rollback remains the responsibility of the durable typed source-evidence adapter.
- Runtime instance and writable-state namespace identities are nonzero and unique per trial arm.
- The current profile requires identical host hardware identity, host kernel digest and architecture across a matched run. Heterogeneous fleets need an explicit stratified profile rather than silent mixing.

## Industry comparison and benchmark protocol

ARL Sandbox v1.3 is a useful external reference for SUT/Harness/Supervisor separation, process/container/microVM tiers, and logical/resource/physical telemetry. It is an emerging specification, not a universal standard; this work does not claim ARL compatibility or an ARL readiness score. Reference: https://arl.transaction.science/spec/arl-s

SandboxBench reports 27 containment challenges across Docker and Kubernetes and substantial attack success in the evaluated settings, including 69–77% on Docker and 40% on Kubernetes. These are attack-task results for its own setup, not interchangeable with this project's tier certification. Reference: https://library.sparai.org/reports/sandboxbench-a-comprehensive-evaluation-framework-for-ai-agent-containment-lmwgda/

SREGym-Lite's current top leaderboard entries reach 100% E2E on a curated 17-fault cohort; that is useful as an episodic smoke test but ceiling-saturated at the top and not a containment result. Reference: https://www.sregym.com/leaderboard

CI-Work reports contextual-integrity violations of 15.8–50.9% across evaluated enterprise workflows. Sandbox isolation does not solve compositional privacy; the separate contextual privacy track (#198) must remain in the benchmark suite. Reference: https://aclanthology.org/2026.acl-industry.103/

### Required benchmark tracks

1. Native SandboxBench-style attack suite, reported separately by tier and attack family.
2. Tier-specific positive/negative boundary tests: process escapes, namespace/mount traversal, cgroup budget bypass, container image/tool sub-sandbox escape, microVM escape and device/accelerator boundary checks.
3. Runtime observer loss/replay/tampering tests, including supervisor-process compromise attempts and missing telemetry.
4. SRE-Marathon-like long-horizon workload runs with independent service outcome measurement.
5. Contextual-integrity/privacy track (#198), including compositional information-flow attacks.
6. Resource/energy efficiency track, reported only with calibrated hardware telemetry and verified-outcome denominators.

Do not aggregate cross-tier scores. Compare agents within the same tier, hardware, host kernel, runtime profile, scenario, workload, resource budget and evaluator. Publish attack success, policy violations, observer completeness, escape attempts, service outcome, energy and cost as separate measures.

## Proposed targets

These are hypotheses for a confirmatory program, not observed results:
- 100% pass on deterministic tier-specific positive/negative controls before any certification label.
- Zero successful sandbox escapes, oracle reads, unauthorized artifact reads or disallowed egress in the sealed challenge cohort; report the exact cluster-level confidence bound, not "zero risk."
- 100% authenticated receipts for credited runs; missing coverage is indeterminate.
- 100% of Tier 3 energy claims carry calibrated host-side samples and hardware identity; otherwise energy is not scored.
- At least 60 adversarial conformance vectors per tier, plus independent red-team attack suites.
- At least 500 independent scenario/topology/fault clusters for confirmatory containment estimates where feasible; requests and repeated executions within one cluster do not inflate the independent denominator.
- For operational efficiency, target at least 20% lower fully loaded cost per verified recovery and 30% less human-active time versus the strongest matched baseline, at non-inferior safety and service outcomes.

## Claim ceiling

The Rust validator proves that supplied records satisfy the encoded tier contract. It does not launch a sandbox, inspect kernel/hypervisor state, verify TPM/HSM attestation, prove hardware isolation, or authenticate Supervisor signatures. Those controls require the external Supervisor adapter, the typed source-evidence verifier and adversarial qualification of the actual runtime. A green CI run is structural conformance evidence only.

Platform Saga remains the episode coordinator; the broker enforces mutation authority; participant-owned ledgers own remote effects; independent workload/business observers own service outcomes; Nixward remains the only host transaction/interlock/journal/recovery authority.


### Latest harness regression correction

The tier fixture carries the runtime challenge-authority identity exactly once in the freeze receipt and once in the derived run ledger. This remains a structural source-binding check; the independent challenge authority and typed evidence verifier authenticate the signed nonce and consume replay state.
