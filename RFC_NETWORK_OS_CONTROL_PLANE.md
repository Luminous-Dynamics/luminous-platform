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

### 4. Configuration authority and runtime ownership are separate

Every managed resource carries two distinct ownership facts:

- **Configuration authority:** which source is entitled to define the desired configuration (for example, NixOS declarative configuration, Nixward's explicit runtime intent, or an identified external authority).
- **Runtime owner:** which process/controller currently maintains the effective kernel state (for example, systemd-networkd, NetworkManager, or a Nixward-owned runtime adapter).

These must not be collapsed into one field. NixOS declarative intent may legitimately be realized by systemd-networkd or NetworkManager. A runtime controller may not infer configuration authority merely because it can observe or mutate the kernel. The M0 planner records the intended and observed values independently and blocks an update when either does not match. Matching state must not hide a controller conflict.

The initial lab uses isolated network namespaces and virtual Ethernet devices created and owned by the test harness. It must not mutate the host's default route, physical interfaces, global firewall policy, or production WireGuard devices.

A later appliance profile must explicitly define the allowed configuration-authority/runtime-owner pair for each resource family and device profile. Two independent controllers must never continually reconcile the same resource. Ownership migration must be an explicit transaction, not an implicit side effect of reconciliation.

### Observation consistency is a first-class proof obligation

Linux does not promise one atomic snapshot across all networking objects. A Netlink dump can be interrupted while the kernel's fast path updates objects; an adapter must inspect dump flags, detect message loss/truncation, and either retry under a bounded policy or mark that resource family incomplete. A successful socket read or zero exit code is not completeness evidence. Completeness evidence must bind the exact target/namespace, resource family, collector build, request/reply boundaries, and loss/interruption indicators. The M0 planner therefore requires per-kind completeness-evidence digests and refuses to infer resource absence unless the caller marks that kind complete. The planner binds the digest only; a separate verifier must check the artifact before any future write-capable adapter can rely on it.

## Scale envelope: from a local router to global network operations

### Direct answer

**Linux can be the foundation for globally deployed networking, but “global hub capacity” is not a single server-size number.** Linux has mature [multi-queue packet distribution (RSS/RPS/RFS/XPS)](https://docs.kernel.org/networking/scaling.html), [network namespaces and VRFs](https://docs.kernel.org/networking/vrf.html), [eBPF/XDP and AF_XDP](https://docs.kernel.org/networking/af_xdp.html), and [interfaces for hardware forwarding offload](https://docs.kernel.org/networking/switchdev.html). Linux-based [SONiC](https://github.com/sonic-net/SONiC) is an existence proof that Linux software can participate in cloud data-center switching, with supported switch ASICs doing line-rate forwarding. These precedents establish feasibility of the platform family, not performance of this project.

We should design for global-scale *fleet management* without routing every customer's packet through one global software hub. Keep forwarding local to each appliance or switch, use distributed routing and local policy enforcement, and build regional/federated management. Global operation is a topology, redundancy, and operations problem as much as a packet-processing problem.

### Three independent capacity dimensions

Never describe capacity only as “devices supported” or “Gbps.” Qualification reports must separate:

1. **Data plane:** throughput by frame size, packet rate, latency/jitter, loss, connection tracking/NAT cost, tunnel overhead, and the exact number/type of ports and NICs.
2. **Routing/control plane:** number of routes/prefixes, peers and sessions, update/withdrawal churn, convergence and failover times, CPU/memory consumption, and forwarding information base (FIB) programming behavior.
3. **Management/fleet plane:** managed appliances, sites, tenants, concurrent operators, telemetry/event rate, queued changes, change completion rate, audit retention, regional outage behavior, and coordinator recovery.

A high-throughput forwarding appliance may manage only a few devices; a fleet controller may manage thousands of low-throughput routers without forwarding any user traffic itself. These are distinct capacity claims.

### Proposed qualification ladder

Every numeric band below is a **target for future qualification, not a current capability or benchmark result**. Support means a specific hardware, NIC/driver, kernel, configuration, and workload matrix has passed its evidence gates.

| Tier | Target environment | Indicative profile | Architectural expectation | Evidence required before claiming support |
|---|---|---|---|---|
| N0 | Developer/CI lab | Network namespaces and QEMU; synthetic virtual links | Read-only inventory/planning first; then isolated namespace-only mutation | Exact-head unit/property tests, failure injection, host-isolation proof |
| N1 | Home/small office/branch | 1/2.5/10-GbE-class interfaces; tens to hundreds of endpoints as a test profile | Linux IP stack, nftables, DHCP/DNS, VLANs, WireGuard, optional dual WAN | Named NIC/device matrix, packet-size sweep, VPN/firewall/NAT tests, rollback/reboot recovery |
| N2 | Multi-site enterprise/campus | 10/25/100-GbE-class interfaces; hundreds to thousands of endpoints as a test profile | VRFs/segmentation, dual-stack policy, HA gateways, FRRouting for BGP/OSPF where required, centralized but locally enforced intent | Route/peer scale tests, convergence, failover, per-zone traffic, redundant-device tests |
| N3 | Data-center/edge/colo | 25/100/400-GbE-class ports as hardware-qualified target profiles | NIC queue/NUMA tuning, RSS/RPS/RFS/XPS, XDP/AF_XDP only where useful, switchdev/ASIC offload where supported | Per-SKU hardware tests; throughput *and* minimum-frame packet-rate tests; offload parity, queue/drop counters, thermal/soak tests |
| N4 | Carrier, exchange, or global edge | Multiple routers/locations; many BGP peers and externally learned routes | Multiple forwarding nodes, ECMP, redundant route reflectors/control services, local failover and explicitly bounded route policies | Routing-scale lab, churn/reconvergence, multi-failure tests, policy leak prevention, independent route/device qualification |
| F1–F4 | Global management fleet | Proposed load tests at 10, 100, 1,000, and 10,000 managed nodes, with multiple regions and tenants | Local agents and regional coordinators; bounded queues, backpressure, idempotent jobs, tenant isolation, sharded telemetry and audit storage | Measured event/command rates, backlog recovery, coordinator loss/partition, duplicate/replay tests, fleet-wide staged rollout and rollback |

**Important boundaries:**

- N1–N4 are product/reference profiles to qualify, not commitments that the current M0 planner runs any of these environments. The current planner has no live adapter or executor.
- A port labeled “100 GbE” does not prove 100-Gb/s application throughput, minimum-frame line rate, bidirectional throughput, or encrypted/NAT throughput.
- Hardware-supported switching and routing offload can be essential at high rates. That is still a Linux-centered design, but it is not a claim that the general-purpose CPU forwards every packet in software.
- For carrier/global deployments, integrate proven routing implementations such as FRRouting and qualified switching hardware rather than rewriting BGP or an ASIC's forwarding engine in Rust.

### Why packet rate can be the real limit

Bandwidth hides a crucial variable: packets per second. Assuming ordinary Ethernet accounting and 64-byte minimum frames (including FCS), plus 8 bytes of preamble/SFD and 12 bytes of inter-frame gap, line-rate traffic is approximately:

| Link rate | Minimum-frame rate at line rate |
|---:|---:|
| 1 Gb/s | 1.49 million packets/s |
| 10 Gb/s | 14.88 million packets/s |
| 100 Gb/s | 148.81 million packets/s |
| 400 Gb/s | 595.24 million packets/s |

This is **the offered on-wire packet rate**, not measured Linux forwarding performance. Real workloads vary by packet size, flow distribution, offloads, tunnel encryption, firewall/NAT rules, route lookups, and hardware. It illustrates why one throughput benchmark using large packets is not a credible global-scale qualification.

Linux networking scales through complementary mechanisms: NIC multi-queue/RSS distributes receives, RPS/RFS/XPS can steer work in software, and [AF_XDP](https://docs.kernel.org/networking/af_xdp.html) can provide specialized high-performance userspace paths when driver support and workload justify them. For switch hardware, [switchdev](https://docs.kernel.org/networking/switchdev.html) can offload supported bridge/FIB operations into the device. [FRRouting](https://frrouting.org/doc/) provides an established routing suite; its documentation describes route reflectors and the use of multiple reflectors to avoid a single point of failure. Each feature is capability- and device-dependent; none is an automatic speed switch.

### Benchmark and qualification protocol

Use a test stack with deliberately different roles:

- [RFC 2544](https://www.rfc-editor.org/rfc/rfc2544) provides a recognized baseline for forwarding throughput, frame loss, and latency measurements; adapt its methodology to the tested environment and newer applicable benchmarking guidance.

- **Correctness/failure:** NixOS VM and namespace labs, deterministic synthetic topology, property tests for planner invariants, fault injection, restart and recovery tests.
- **Basic TCP/UDP throughput:** iperf3 or equivalent, clearly separating endpoint throughput from forwarding benchmarks.
- **High-rate packets and realistic flows:** evaluate TRex (DPDK-based) or equivalent hardware traffic generation; use kernel pktgen for targeted kernel-path tests where appropriate. A software generator must itself be shown not to be the bottleneck.
- **Comparable device benchmarking:** adapt RFC 2544 test concepts for throughput, frame loss, and latency. Publish exact run configuration and repeat counts rather than selecting the best run.

For each target SKU/profile, test IPv4 and IPv6, TCP and UDP, minimum/IMIX/jumbo frame mixes, routing-only, firewall, NAT/conntrack, WireGuard/IPsec if included, VLAN/VRF isolation, realistic concurrent flows, connection creation rate, route churn, and failures. Record sustained and burst throughput, packets/s, loss, latency distribution, CPU per core, IRQ/softirq utilization, memory, NIC/driver/kernel version, offload settings, queue drops, power/thermal state, and firmware. Include unidirectional and bidirectional tests.

Pass criteria must be chosen before running the tests and tied to a specific product tier. Report p50/p95/p99 latency and loss at stated offered loads, not only a peak Gbit/s number. Add soak tests and failure/reconvergence tests appropriate to the tier. A benchmark pass qualifies only that tested hardware/software/workload envelope; it does not certify arbitrary Linux hardware.

### Global fleet topology

For F1–F4, do not make a single controller mandatory for live forwarding. Preferred shape:

- **Device-local enforcement and recovery:** every device maintains last-known-good authorized policy and continues normal local forwarding during loss of the management path.
- **Regional coordinators:** tenant-scoped desired state, bounded work queues, idempotent per-target changes, versioned rollout waves and regional observability.
- **Federation/root coordination:** global inventory, policy distribution, trust relationships, conflict detection, and cross-region evidence aggregation; not a transit path for each user packet.
- **Hierarchical failure policy:** partitioned regions stop unsafe writes or hold last-known-good state; they do not wait indefinitely for a global quorum to forward packets.
- **Blast-radius controls:** canaries, maintenance windows, concurrent-change detection, rate limits, per-site rollback, and an independent recovery channel for gateway changes.

The actual F1–F4 node counts, event rates, data retention, availability objectives, and region boundaries remain open design/qualification parameters. Do not promote the illustrative node-count ladder into a product SLA before a measured fleet simulation exists.

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
4. **Recompute and compare** the candidate plan from the exact snapshot and desired-state inputs. Structural validation and a supplied digest do not prove that a serialized plan is derived from those inputs.
5. **Authorize** the exact target and recomputed plan through the established Nixward authority path.
6. **Re-observe** immediately before application; reject stale pre-state rather than silently replanning after authorization.
7. **Apply** only the previously authorized operations, through a narrow adapter.
8. **Verify** by querying the resulting kernel/device state and applicable reachability invariants; exit code alone is insufficient.
9. **Promote or recover** with an explicit terminal state.

A multi-step network mutation is **not automatically atomic** just because a planner calls it a transaction. The implementation must classify operations as batch-atomic where the underlying API supports it, otherwise use ordered operations and bounded compensation. If rollback fails, concurrent changes make recovery unsafe, or post-state is ambiguous, report `Indeterminate/RecoveryRequired`; never claim success.

For disruptive changes, require commit-confirm with a recovery watchdog independent of the mutating process. The watchdog must be bound to the transaction and must not overwrite newer operator changes during recovery. Remote devices require a viable out-of-band or local recovery path before a route, firewall, or interface change that could sever management access is permitted.

The daemon should run with the narrowest feasible privileges and a small privileged mutation boundary. Never pass user-controlled values through a shell. Secrets such as WireGuard private keys must be excluded from logs, receipts, diagnostics, crash reports, and support bundles.

## Dependency and interface research snapshot (2026-10-12)

These links are initial evaluation inputs, not blanket endorsements. Pin dependencies and review their licenses, MSRV, release history, feature coverage, and transitive dependencies before implementation.

- [Linux Netlink introduction](https://docs.kernel.org/userspace-api/netlink/intro.html) explains the kernel/userspace protocol and its message model. Its dump-consistency section warns that a dump can be interrupted and inconsistent (including missing objects); userspace must detect `NLM_F_DUMP_INTR` and retry rather than mark the inventory complete.
- [Linux Netlink protocol specifications](https://docs.kernel.org/userspace-api/netlink/specs.html) documents machine-readable YAML protocol specifications and generated interfaces. Where a mature crate does not cover a needed family, evaluate the kernel's published YAML spec and generated/YNL approach instead of inventing wire layouts.
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
- Every planned resource has explicit configuration authority and runtime owner, capability, observed pre-state, intended state, and postcondition.
- Deterministic tests demonstrate stale-state rejection, conflict detection, stable plan digests, candidate-plan tamper detection by recomputation, and explicit unknown/indeterminate outcomes.
- A Phase 2 actuator cannot target outside the test harness's owned namespace/resources.
- No integration is described as implemented or qualified until its exact-head evidence is available.
