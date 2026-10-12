# RFC: Luminous Network Control Plane and Linux-Based Network OS

- **RFC:** LNOS-0001
- **Status:** Proposed — architecture review only
- **Date:** 2026-10-12
- **Scope:** Luminous Platform, Nixward, and Luminous Edge
- **Implementation status:** Not implemented or qualified by this document
- **Decision sought:** Approve the direction and the lab-only discovery milestone, not production network mutation.

## Summary

Build an open, Rust-first network control plane and a NixOS-based appliance profile by composing proven Linux networking components. Do not rewrite the Linux TCP/IP stack, drivers, firewall, VPN, or routing protocols as a starting point.

The first product milestone is a **read-only observer and deterministic planner**. The first mutation milestone must operate only inside an explicitly created virtual network lab. Physical-host writes remain disabled until safety evidence, rollback/recovery, ownership conflict detection, and a remotely recoverable test profile are qualified.

Provisional service name: `luminous-networkd`. The name is not yet a reserved crate or repository name.

## Why this belongs in the platform

The Luminous Platform already describes a network layer containing `xenia-peer`, `xenia-wire`, and the Mycelix mesh. Nixward already defines a consequential-change lifecycle:

> observe → plan → validate → authorize → snapshot → apply → verify → promote/recover

This RFC applies that existing transaction discipline to Linux network resources; it does not create a parallel authorization or transaction system. See [Nixward's canonical transaction architecture](https://github.com/Luminous-Dynamics/nixward/blob/main/docs/SYSTEM_TRANSACTION_ARCHITECTURE.md) and the [Luminous Platform architecture](https://github.com/Luminous-Dynamics/luminous-platform).

## Goals

1. Model intended and observed network state with typed Rust structures and explicit provenance.
2. Use Linux's established networking APIs and implementations through narrow, testable adapters.
3. Make every consequential mutation attributable to an exact plan, target, pre-state, authority decision, and verified post-state.
4. Fail closed when ownership, preconditions, authorization, or verification is ambiguous.
5. Keep packet forwarding functional if Symthaea, Mycelix, the platform management service, or the management network is unavailable.
6. Produce a portable open management contract over time, without requiring all vendors to implement Luminous internals.

## Non-goals for the first release

- Replacing Linux TCP/IP, drivers, or the kernel.
- Implementing new cryptography, a new VPN protocol, or BGP/OSPF from scratch.
- Placing distributed consensus or the Holochain data path in packet forwarding.
- Letting an AI model, confidence/Phi score, language model, or natural-language request authorize a mutation.
- Managing every interface and network service on a host that has other active network managers.
- Claiming support for arbitrary hardware before device-specific qualification.
- Shipping automatic network optimization before bounded policy, rollback, and postcondition tests exist.

## Proposed architecture

### 1. Linux remains the data plane

Reuse the Linux kernel IP stack, drivers, namespaces, bridges, VLANs, traffic control, and established network features. Compose rather than replace mature components:

- **Links, addresses, routes, rules, neighbours:** Linux Netlink APIs, initially evaluated through the Rust `rtnetlink` crate.
- **Firewall/NAT:** nftables; preserve nftables' batch semantics and verify the effective ruleset after application.
- **Routing protocols:** integrate FRRouting when dynamic routing is required; the Luminous control plane expresses and verifies intent rather than reimplementing BGP, OSPF, or IS-IS.
- **Encrypted tunnels:** reuse WireGuard and its established kernel/userspace control interface.
- **Network acceleration:** no custom XDP/eBPF behavior in the first milestone. Evaluate Aya only for a scoped, measurable need after baseline behavior is correct.

Adapters must be hidden behind internal traits so one crate, protocol, or subprocess decision cannot become a permanent architecture constraint. The first implementation should explicitly document the selected adapter and exact pinned dependency versions.

### 2. Rust control plane

Proposed modules (logical boundaries; not yet a committed crate layout):

- `network-model`: versioned desired-state and observed-state types; explicit `Unknown`, `Observed`, `Desired`, and `Derived` provenance where applicable.
- `network-inventory`: read-only discovery, link identity, capability discovery, and event collection.
- `network-planner`: pure deterministic plan generation from desired state, observed state, and declared capabilities.
- `network-policy`: invariant checking, address/routing constraints, segmentation intent, and ownership rules.
- `network-adapters-linux`: narrow Netlink, nftables, and WireGuard adapters.
- `network-transaction`: pre-state binding, authorization boundary, staged apply, postcondition verification, and recovery state.
- `network-evidence`: structured, privacy-aware receipts and operator-visible explanations.
- `network-api`: local IPC and later a versioned remote API; never expose a general-purpose privileged shell.

These are proposed boundaries, not claims that the crates exist.

### 3. Responsibilities across Luminous components

- **Nixward:** own the consequential transaction model, explicit authority checks, local execution boundary, and recovery discipline. Reuse its existing contracts instead of inventing a second authorization system.
- **Luminous Platform / Luminous Edge:** package and compose the network modules into optional NixOS profiles only after qualification.
- **Mycelix:** later provide federation, identity, delegated capabilities, and portable trust/evidence semantics. Local enforcement and recovery must work during disconnection; no remote quorum may be required to forward packets.
- **Symthaea:** initially consume telemetry and propose explanations or plans. It is advisory only. No cognitive output is mutation authority.
- **Xenia:** retain its separate transport/peer responsibilities; use authenticated, versioned interfaces rather than coupling the network daemon to internal transport implementation details.

### 4. Resource ownership is mandatory

Each managed resource must have one declared writer/owner. The controller must detect, not silently fight, configurations owned by NixOS-generated units, systemd-networkd, NetworkManager, another agent, or an operator. If ownership cannot be established, operate read-only and report a conflict.

The initial lab uses isolated network namespaces and virtual Ethernet devices created and owned by the test harness. It must not mutate the host's default route, physical interfaces, global firewall policy, or production WireGuard devices.

A later appliance profile must explicitly define how NixOS declarative configuration and runtime reconciliation divide ownership. Two independent controllers must never continually reconcile the same resource.

## Transaction and safety contract

Every write-capable operation must bind to an immutable transaction intent containing at least:

- target and namespace identity;
- exact resource set and requested mutations;
- observed pre-state and its digest / generation;
- desired-state and plan digest;
- required capability and policy checks;
- expiry and replay-resistant transaction identity;
- rollback or recovery plan, including limits;
- expected postconditions and verification procedure.

The required lifecycle is:

1. **Observe** target-side state and label provenance.
2. **Plan** deterministically without mutation.
3. **Validate** schema, capabilities, ownership, invariants, and recovery feasibility.
4. **Authorize** the exact target and plan through the established Nixward authority path.
5. **Re-observe** immediately before application; reject stale pre-state rather than silently replanning after authorization.
6. **Apply** only the previously authorized operations, through a narrow adapter.
7. **Verify** by querying the resulting kernel/device state and applicable reachability invariants; exit code alone is insufficient.
8. **Promote or recover** with an explicit terminal state.

A multi-step network mutation is **not automatically atomic** just because a planner calls it a transaction. The implementation must classify operations as batch-atomic where the underlying API supports it, otherwise use ordered operations and bounded compensation. If rollback fails, concurrent changes make recovery unsafe, or post-state is ambiguous, report `Indeterminate/RecoveryRequired`; never claim success.

For disruptive changes, require commit-confirm with a recovery watchdog independent of the mutating process. The watchdog must be bound to the transaction and must not overwrite newer operator changes during recovery. Remote devices require a viable out-of-band or local recovery path before a route, firewall, or interface change that could sever management access is permitted.

The daemon should run with the narrowest feasible privileges and a small privileged mutation boundary. Never pass user-controlled values through a shell. Secrets such as WireGuard private keys must be excluded from logs, receipts, diagnostics, crash reports, and support bundles.

## Dependency and interface research snapshot (2026-10-12)

These links are initial evaluation inputs, not blanket endorsements. Pin dependencies and review their licenses, MSRV, release history, feature coverage, and transitive dependencies before implementation.

- [Linux Netlink introduction](https://docs.kernel.org/userspace-api/netlink/intro.html) explains the kernel/userspace protocol and its message model.
- [Linux Netlink protocol specifications](https://docs.kernel.org/userspace-api/netlink/specs.html) documents machine-readable YAML protocol specifications and generated interfaces.
- [Linux nftables Netlink specification](https://docs.kernel.org/netlink/specs/nftables.html) documents batch-begin/batch-end and rule/table operations.
- [Linux WireGuard Netlink specification](https://docs.kernel.org/netlink/specs/wireguard.html) documents the device and peer control interface.
- [`rtnetlink` on docs.rs](https://docs.rs/crate/rtnetlink/0.23.0) listed v0.23.0 (released 2026-08-18) at research time and covers links, addresses, neighbours, and routes. Its published changelog reports Rust 1.75 as the minimum supported version; Luminous should still test and pin a known-good version.
- [Aya on docs.rs](https://docs.rs/aya/0.14.0/aya/) listed v0.14.0 at research time. It is a candidate for future eBPF/XDP work, not a dependency required by the first milestone.
- [FRRouting](https://frrouting.org/) is an established routing-protocol suite; integrate it for routing protocols rather than reimplementing those protocols in the first release.

No throughput or latency claims are made. No Rust networking dependency has been qualified against the Luminous target kernels by this RFC.

## Delivery phases and gates

### Phase 0 — Design and threat model (this RFC)

**Output:** agreed responsibilities, resource ownership rules, threat model, first profile, and measurable gates.

**Gate:** no production mutation is enabled by accepting this RFC.

### Phase 1 — Read-only inventory and pure planner

Build typed state and a deterministic planner. Collect observations from a controlled virtual lab; render proposed diffs. Repeated runs with identical inputs must yield the same normalized plan digest. Unknown or unsupported resources must remain explicit and must not be silently omitted.

**Gate:** tests establish deterministic planning, provenance, stable normalization, schema rejection, and correct conflict reporting. No host mutation path is present.

### Phase 2 — Namespace-only actuator

Add controlled apply and verification strictly within resources created by an isolated test harness. Start with link creation, address configuration, routes, and a dedicated nftables table/chain. Every mutation must be limited to the lab's namespace and owned resources.

**Gate:** fault-injection tests cover stale pre-state, duplicate/reordered notifications, rejected Netlink requests, partial apply, process kill at each lifecycle phase, failed rollback, and restart/recovery. Test that host interfaces/default routes/global firewall state remain unchanged.

### Phase 3 — Narrow appliance profile in QEMU

Package an optional NixOS module and run it in disposable QEMU instances with a known-good baseline and a separate recovery path. Add one supported network profile at a time. Keep default production mutation disabled until qualification evidence is accepted.

**Gate:** reproducible image, exact-head evidence, upgrade/rollback tests, management-connectivity commit-confirm test, reboot recovery, security review, and a documented capability matrix.

### Phase 4 — Physical hardware qualification

Qualify named device models, NICs, firmware/kernel combinations, and recovery paths individually. Virtual success is not evidence of physical hardware compatibility.

### Phase 5 — Open management contract and ecosystem

Only after the first implementation is useful, publish a versioned and implementation-neutral network-intent and conformance-test specification. Define compatibility and downgrade rules; do not expose Luminous-specific internal types as the standard. Build vendor adapters and fleet management against that contract.

## Required first test matrix

| Case | Required result |
|---|---|
| Same desired and observed state | No-op plan; no privileged write |
| Unknown field or unsupported capability | Reject or report unsupported; never guess |
| Resource already owned by another manager | Read-only conflict |
| Observed state changes after authorization | Reject as stale; require a new plan and authorization |
| Netlink/API call fails | Record partial state; verify before deciding recovery |
| Process dies before, during, or after apply | Restart discovers actual state; no false success |
| Rollback fails or concurrent changes appear | Explicit recovery-required state; no blind overwrite |
| Firewall/route change risks management reachability | Block unless tested commit-confirm/out-of-band recovery contract is satisfied |
| Secrets are present | Redacted from every evidence/logging path |
| AI/suggestion proposes a change | No privileged effect without independent authorization |
| Lab test is run | Host physical links, main routing table, and global firewall remain unchanged |

All pass claims must be bound to the exact commit, configuration, kernel, and test environment. A queued workflow or successful process launch is not a pass.

## Risks and open decisions

1. **Scope:** a full-featured router, MSP stack, switch OS, and mesh fabric are different products. The first increment is the controller, not all products at once.
2. **Ownership conflicts:** NixOS, systemd-networkd, NetworkManager, and other agents can compete for the same resources. Read-only conflict behavior is required.
3. **Remote lockout:** a controller failure can break the path needed to repair networking. Recovery must not depend solely on that same network.
4. **Adapter maturity:** Netlink coverage and behavior vary by resource and kernel. Keep adapters replaceable and qualify the exact deployed versions.
5. **Hardware/driver variability:** support claims require per-profile evidence.
6. **Evidence and privacy:** topology and telemetry can reveal sensitive operational details. Receipts should prove the required properties without exporting credentials or unnecessary packet contents.

Before moving beyond Phase 1, confirm the initial deployment profile (recommended: NixOS-based small-office/home-office router in a virtual lab) and the exact existing Nixward authorization/receipt interfaces to consume.

## Acceptance criteria for this RFC's first implementation milestone

- The first deliverable is a read-only inventory plus pure planner, not a production router.
- No physical-host networking mutation is reachable from the Phase 1 build.
- Every planned resource has explicit owner, capability, observed pre-state, intended state, and postcondition.
- Deterministic tests demonstrate stale-state rejection, conflict detection, stable plan digests, and explicit unknown/indeterminate outcomes.
- A Phase 2 actuator cannot target outside the test harness's owned namespace/resources.
- No integration is described as implemented or qualified until its exact-head evidence is available.
