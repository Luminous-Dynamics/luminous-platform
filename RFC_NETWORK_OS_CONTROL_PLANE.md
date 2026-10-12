# RFC: Luminous Network Control Plane and Linux-Based Network OS

- **RFC:** LNOS-0001
- **Status:** Proposed — architecture review only
- **Date:** 2026-10-12
- **Scope:** Luminous Platform, Nixward, and Luminous Edge
- **Implementation status:** A pure M0 planner is proposed in [Nixward PR #65](https://github.com/Luminous-Dynamics/nixward/pull/65); it has not been compiled, tested, merged, or qualified.
- **Decision sought:** Approve the direction and the lab-only discovery milestone, not production network mutation.

## Summary

Build an open, Rust-first network control plane and a NixOS-based appliance profile by composing proven Linux networking components. Do not rewrite the Linux TCP/IP stack, drivers, firewall, VPN, or routing protocols as a starting point.

The first product milestone is a **read-only observer and deterministic planner**. The first mutation milestone must operate only inside an explicitly created virtual network lab. Physical-host writes remain disabled until safety evidence, rollback/recovery, ownership conflict detection, and a remotely recoverable test profile are qualified.

Do not start by creating a second daemon. The first increment should be a read-only library/planner integrated with Nixward's existing transaction boundaries. A separate `luminous-networkd` process is a later option only if a stable API and a privilege-separation case justify it.

## Why this belongs in the platform

The Luminous Platform already describes a network layer containing `xenia-peer`, `xenia-wire`, and the Mycelix mesh. Nixward already defines a consequential-change lifecycle:

> observe → plan → validate → authorize → snapshot → apply → verify → promote/recover

**Repository inspection found existing Nixward networking work that this RFC must compose with, not duplicate:**

- [`network_covenant.rs`](https://github.com/Luminous-Dynamics/nixward/blob/main/src/network_covenant.rs) defines workload-level network policy/capability contracts.
- [`network_enforcement.rs`](https://github.com/Luminous-Dynamics/nixward/blob/main/src/network_enforcement.rs) builds cgroup/BPF workload enforcement plans and kernel/target proof structures.
- [`network_runtime_lab.rs`](https://github.com/Luminous-Dynamics/nixward/blob/main/src/network_runtime_lab.rs) and the [runtime-lab harness](https://github.com/Luminous-Dynamics/nixward/tree/main/tests/network-runtime-lab) establish an isolated NixOS VM test/evidence path for that enforcement backend.
- [`network_activation.rs`](https://github.com/Luminous-Dynamics/nixward/blob/main/src/network_activation.rs) binds plan/readiness/target artifacts to a separate owner-authorized activation intent.
- [`network_telemetry.rs`](https://github.com/Luminous-Dynamics/nixward/blob/main/src/network_telemetry.rs) defines a metadata-only evidence stream and explicitly excludes payload capture and mutation authority.

Those modules concern **workload/network-flow enforcement and its evidence**. This RFC concerns the complementary **device/topology reconciliation plane**: links, addresses, routes, VLANs/bridges, device-facing firewall policy, and eventual fleet-wide network intent. It must reuse shared authority/receipt contracts where appropriate, but must not re-label workload BPF evidence as proof of router/device qualification, duplicate the existing runtime lab, or create a second policy authority.

The broader Luminous Platform also has draft proposals for [government/regulatory network architecture (PR #20)](https://github.com/Luminous-Dynamics/luminous-platform/pull/20), a [hardware portfolio and qualification package (PR #22)](https://github.com/Luminous-Dynamics/luminous-platform/pull/22), and [self-hosted workload/service contracts (PR #28)](https://github.com/Luminous-Dynamics/luminous-platform/pull/28). They remain drafts; the device-control design should align with them rather than duplicating product taxonomy, segmentation contracts, hardware qualification, or workload declarations.

An initial implementation slice is proposed in [Nixward PR #65](https://github.com/Luminous-Dynamics/nixward/pull/65). It defines validated input/snapshot types and a pure decision planner only. It does **not** collect live kernel state, calculate semantic route/firewall diffs, execute changes, or provide a device adapter. Its current validation status is unverified because tests have not been run; it must not be treated as a pass or qualification result.

This RFC applies Nixward's existing transaction discipline to the new resource domain; it does not create a parallel authorization or transaction system. See [Nixward's canonical transaction architecture](https://github.com/Luminous-Dynamics/nixward/blob/main/docs/SYSTEM_TRANSACTION_ARCHITECTURE.md) and the [Luminous Platform architecture](https://github.com/Luminous-Dynamics/luminous-platform).

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

- **Nixward:** keep the existing workload-level cgroup/BPF enforcement path and its activation/evidence contracts authoritative for that domain. Add a distinct device/topology reconciliation module behind the existing transaction, authorization, and receipt boundary; do not fork a second transaction engine or create a second network policy authority.
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

### Phase 1 — Existing-contract map, read-only inventory, and pure planner

First map the exact public APIs and evidence contracts in Nixward's existing `network_covenant`, `network_enforcement`, `network_activation`, `network_runtime_lab`, and `network_telemetry` modules. Record which contracts are reusable and which are deliberately workload-scoped. Do not change their backend IDs or interpret their attestations as device-controller qualification.

Then build a separate, read-only device/topology inventory and deterministic planner. Collect observations from a controlled virtual lab; render proposed diffs. Repeated runs with identical normalized inputs must yield the same plan digest. Unknown or unsupported resources must remain explicit and must not be silently omitted. The planner must identify an existing product/site segmentation schema only after its owning proposal is accepted, not invent a competing zone model.

**Gate:** a written integration map plus tests establish deterministic planning, provenance, stable normalization, schema rejection, and correct conflict reporting. No host mutation path is present.

### Phase 2 — Separate device-runtime lab and namespace-only actuator

Do not reuse the existing workload BPF runtime-lab attestation for this new resource domain. Add a separately named and versioned device/topology lab contract and harness, using the existing NixOS VM test pattern where useful. Add controlled apply/verification strictly within resources created by that harness. Start with virtual link creation, addresses, routes, and an isolated nftables table/chain. Every mutation must be limited to the harness-owned namespace and resources.

**Gate:** fault-injection tests cover stale pre-state, duplicate/reordered notifications, rejected Netlink requests, partial apply, process kill at each lifecycle phase, failed rollback, and restart/recovery. Test that host interfaces/default routes/global firewall state remain unchanged. The new report must have a distinct backend identity, schema, and verifier.

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

Before moving beyond Phase 1, map the exact Nixward transaction/authorization/receipt APIs and resolve overlaps with the open Luminous network segmentation and product-profile proposals. Recommended first profile: a NixOS-based small-office/home-office router in a virtual lab; it remains a proposed target, not a product commitment.

## Acceptance criteria for this RFC's first implementation milestone

- The first deliverable is a read-only inventory plus pure planner, not a production router.
- No physical-host networking mutation is reachable from the Phase 1 build.
- Every planned resource has explicit owner, capability, observed pre-state, intended state, and postcondition.
- Deterministic tests demonstrate stale-state rejection, conflict detection, stable plan digests, and explicit unknown/indeterminate outcomes.
- A Phase 2 actuator cannot target outside the test harness's owned namespace/resources.
- No integration is described as implemented or qualified until its exact-head evidence is available.
