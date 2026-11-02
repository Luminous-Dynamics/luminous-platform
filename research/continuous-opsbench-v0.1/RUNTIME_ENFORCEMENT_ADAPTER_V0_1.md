# Runtime enforcement adapter design v0.1

## Objective and assurance boundary

This document specifies an implementation path for the runtime receipt contract in `src/runtime.rs`. The contract is a structural verifier. A green Rust test suite proves that supplied records obey the data model; it does not prove the kernel, hypervisor, object store, network boundary, or observer actually enforced or measured the claimed behavior.

The required proof chain is:

1. the run freeze commits the policy, environment, observer and challenge-authority identities;
2. a separate challenge authority issues a fresh one-use challenge for each planned trial × arm;
3. a sandbox launcher creates a unique runtime instance and writable namespace;
4. a runtime observer starts before the evaluated process tree and remains active until the final child exits;
5. the observer emits a source-authenticated record with complete coverage and exact event-stream commitments;
6. the typed evidence adapter verifies challenge and observer statements, signature, subject, policy digest, freshness, source/control domain and durable sequence replay;
7. `validate_runtime_attestations_from_events` derives the candidate/baseline trial ledger from the exact event records and requires one matching challenge and attestation for every arm.

The challenge receipt is a distinct signed statement from the challenge authority. The observer cannot self-assert that a nonce was issued. Both parties must bind the same nonce, run, trial, arm, scenario, identities, policy digests, runtime instance, namespace and validity interval. The structural validator checks these bindings but delegates cryptographic authentication to the existing source-evidence gate (#190/#191).

## Linux reference architecture

### 1. Enforcement layer

For host-native Linux workloads, use multiple independently meaningful boundaries:

- **Landlock** for per-process filesystem and supported network restrictions, with the exact available ABI and handled access rights probed and pinned. Landlock rules are additive restrictions; unsupported required controls must fail closed rather than silently downgrade.
- **Mount, user, PID, IPC and network namespaces** to remove ambient host paths, credentials and network reachability. A namespace is not itself a complete access-control boundary.
- **seccomp-BPF** to restrict unnecessary syscall surface, especially namespace creation, mount, ptrace, keyring, BPF/perf and device operations not needed by the workload.
- **cgroup v2** for per-trial CPU, memory, process-count and I/O budgets, process-tree membership, and kill/cleanup.
- **Network namespace + default-deny egress gateway**. Use a narrow, explicit allowlist and record DNS queries, resolved addresses, connection attempts, TLS/HTTP destination metadata and response digests at the gateway. Do not treat a DNS allowlist alone as an egress policy.
- **Separate artifact service identities**. The agent receives a read-only case/evidence identity only. Oracle bytes and oracle credentials remain in the evaluator trust domain. A file not mounted into the sandbox is stronger protection than hoping to observe and reject every attempted read.

For higher-risk untrusted code, compare a microVM boundary (Firecracker/KVM) or a userspace-kernel sandbox (gVisor) against a host-native namespace/Landlock/seccomp configuration. Record the exact runtime and configuration digests. Do not present these runtime families as equivalent security levels.

References:
- Linux Landlock userspace access control: https://docs.kernel.org/userspace-api/landlock.html
- Landlock system-wide audit and trace semantics: https://www.kernel.org/doc/html/latest/admin-guide/LSM/landlock.html
- Landlock trace events: https://kernel.org/doc/html/next/trace/events-landlock.html
- Firecracker defense-in-depth isolation: https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md
- gVisor security model: https://gvisor.dev/docs/architecture_guide/security/

### 2. Observer layer: do not confuse denial logs with complete access telemetry

Landlock audit/trace events are valuable for domain lifecycle, rule creation, rule enforcement and denials. They do **not** form a complete success-event ledger for every successful filesystem or network read. Some speculative NOAUDIT checks generate no audit/trace event, audit records may be filtered or dropped when queues are full, and Landlock tracepoints are disabled by default unless explicitly enabled.

Consequently:
- Landlock is an enforcement source and a denial-evidence source, not the only successful-access observer.
- Use a host-side observer attached to the exact runtime cgroup/VM identity before launch. Prefer stable tracepoint/LSM hooks where available; pin kernel release, config, BTF identity, program digest and attach points in the adapter/environment evidence.
- Observe successful and denied file opens/reads, object-store reads, process execution/child creation, mount/namespace changes, socket creation/connect/send, DNS resolution, and cgroup membership.
- For a VM, collect host-side VM lifecycle, vCPU, virtual NIC/TAP and artifact-service events; if guest logs are also used, they are supplementary and not the sole proof.
- Track per-CPU/per-ring-buffer lost-event counters, BPF map insertion failures, audit backlog/drops, userspace queue overflow, process attach gaps and observer restarts. Any unbounded or unexplained gap means coverage is partial/unknown, never zero accesses.
- Maintain a monotonic event sequence and a separate wall-clock/uncertainty estimate. Clock overlap within the declared uncertainty must not be misreported as an ordering violation.

The observer must live outside the agent's user, mount, PID and network namespaces and must not share the agent or evaluator control domain. Access to BPF, audit and trace output is privileged and can expose paths, process identities and network metadata; isolate and protect that data as sensitive operational evidence.

### 3. State and artifact separation

- Allocate a unique runtime instance and writable namespace for every `(run, trial, arm)`.
- Use distinct per-arm writable overlays, homes, temp dirs, package caches, compiler caches, browser profiles, credential stores and local service state.
- Make common scenario inputs immutable and read-only.
- Do not reuse writable cache or snapshot state between candidate and baseline. An explicit persistent-agent benchmark is a separate track whose persistence scope is frozen and reported as a capability, not an accidental confound.
- Keep private oracle objects and credentials outside the agent's mount namespace and outside the agent's artifact-service authorization.
- Record exact mount tables, mount IDs, device/inode identities where appropriate, object-store principal and object version/digest, and namespace identity. A pathname alone is not sufficient to establish object identity.

### 4. Fresh challenge protocol

One authority-issued challenge exists for every planned `(trial, arm)`:
- issued strictly after the manifest freeze and before runtime measurement begins;
- bound to run/trial/arm, arm identity digest, scenario commitment, manifest commitment, evaluator, common environment/resource budget, state/artifact/egress policies, runtime instance and writable namespace;
- names the exact observer source, adapter digest and control domain expected to produce the runtime receipt;
- carries a cryptographically random nonce, a source-authenticated receipt digest, monotonic authority sequence and bounded expiry;
- is consumed exactly once by the observer receipt, with matching challenge digest, nonce, issue/expiry times and identities.

The challenge authority and runtime observer are separate control domains. The source-evidence adapter verifies their signatures and enforces durable anti-replay/anti-rollback. The Rust validator checks one-to-one joins and sequence/freshness constraints but does not implement cryptography or persist replay state.

Do not derive the challenge receipt from the observer attestation. That defeats the separation this protocol exists to establish.

### 5. Evidence bundle

The observer receipt commits to:
- runtime instance, writable namespace and process-tree evidence;
- mount/namespace configuration and policy-enforcement evidence;
- successful and denied filesystem/object/network access streams;
- resolver and egress-gateway stream;
- audit/trace/eBPF event stream and all drop/overflow counters;
- observer version, kernel/build/BTF identity, attach-program digests and clock uncertainty.

The corresponding artifact store retains the raw records under a restricted evidence principal. The receipt digest binds the exact serialized record; the source adapter verifies it. The structural Rust module only checks nonzero/present digest fields and cross-record equality.

## Adversarial runtime test matrix

At minimum, test:
- oracle path mounted read-only, unmounted, bind-mounted under an alternate path, exposed through inherited file descriptor, exposed through an object-store token, and accessed through a debug endpoint;
- allowed and disallowed DNS answers, direct-IP egress, IPv6, UDP/QUIC, loopback, Unix sockets, proxy bypass, and DNS rebinding;
- child-process and daemonized-process escape, fork/exec storms, reparenting and cgroup migration attempts;
- mount namespace escape, user namespace creation, ptrace, procfs/sysfs/dev access, host socket and capability access;
- candidate-to-baseline cache contamination, reused overlay, hard links, shared tmp, shared package/browser/compiler cache, and snapshot restore;
- eBPF ring-buffer loss, per-CPU sequence gaps, map exhaustion, audit backlog/rate limiting, collector restart, observer killed before final child exit, and partial host reboot;
- duplicate/replayed challenge nonce, copied challenge receipt, expired challenge, wrong arm/instance/namespace, wrong observer source, and challenge-authority sequence rollback.

For every test distinguish attempted, denied, successful, unobserved and unknown. A deny event is evidence that an attempt happened; absence of an event is not evidence of absence unless observer coverage and source counters establish completeness for that event class.

## Performance and benchmark targets

Instrumentation itself must be benchmarked. Report:
- agent and service p50/p95/p99 latency with and without observer;
- CPU/memory overhead, collector throughput, event volume, storage volume, dropped-event fraction and time-to-final-receipt;
- observer coverage under burst load and process churn;
- false indeterminate rate on known-clean controls and detection rate on seeded violations.

Initial engineering targets (not measured results):
- 100% detection of seeded prohibited oracle reads, state carry-over and disallowed egress in the conformance corpus;
- 0 accepted replayed/expired/substituted challenges;
- 0 silent observer-loss cases; every induced collector drop produces an incomplete receipt;
- less than 5% p95 workload latency overhead as a first target, then optimize under equivalent coverage;
- less than 1% indeterminate runtime attestations in the clean-load calibration campaign, without weakening coverage requirements.

The target is not zero overhead at any cost. A faster observer that silently drops events is worse than a slower observer that correctly declares its uncertainty.

## Claim ceiling

Even a complete signed runtime receipt proves only what the trusted observer and enforcement boundary can measure. It does not prove the absence of all hardware side channels or defects in the kernel/hypervisor/observer. Certification should state the exact threat model, kernel/runtime version, isolation boundary, observer coverage, known blind spots, and seeded-violation detection rate. The source evidence must remain independent of the agent.

Architecture ownership remains unchanged: Platform Saga coordinates the episode; the broker gates mutation authority; participant-owned ledgers own remote effect truth; independent workload/service observers own business outcomes; Nixward remains the sole host transaction/interlock/journal/recovery authority.
