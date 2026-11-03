# Luminous Network Fabric (LNF): End-to-End Architecture and Benchmark Contract

**Status:** Research architecture, version 0.1  
**Date:** 2026-11-02  
**Scope:** Device-to-device, community, terrestrial, non-terrestrial, and interplanetary communications.  
**Normative status:** Proposal only. This document establishes no production guarantee, compliance claim, spectrum authorization, carrier agreement, or hardware qualification.

## 1. Executive decision

Luminous Network Fabric (LNF) is an interoperable **network of networks**, not a replacement for IP, 3GPP, Wi-Fi, O-RAN, QUIC, BPv7, BPSec, or CCSDS.

LNF should make heterogeneous links and providers usable through one coherent policy, identity, continuity, operations, and evidence architecture. Preserve existing standards at protocol boundaries. Create new mechanisms only where a demonstrated gap remains.

The first product is not a new cellular radio or satellite constellation. It is a deterministic, explainable **connectivity and continuity fabric** that can run on commodity endpoints and edge gateways, work with existing WANs, and evolve from laboratory networks into authorized community pilots.

### Primary architectural thesis

Optimize for **successful useful communication under declared constraints**, not headline throughput or the fiction that every network is continuously connected.

A user or application declares intent (for example: local-only, interactive, deadline-bound, low-cost, bulk, or durable). A deterministic policy core selects eligible routes and returns an explainable decision. Observed links may change; identity and application-level message intent remain stable. When no live path exists, explicitly durable messages can be retained and forwarded through a standards-based DTN path.

## 2. Design principles and paradigm shifts

1. **Service continuity over connection continuity.** Preserve useful work across handovers, outages, partitions, device sleep, and long-delay contacts. Do not promise every live socket survives a path change.
2. **Intent before optimization.** Applications declare latency/deadline, reliability, byte budget, cost ceiling, energy budget, locality, privacy, and expiry requirements. The router must not infer intent from payloads, browsing history, or opaque AI guesses.
3. **Deterministic safety, adaptive optimization.** Hard constraints, authorization, quotas, and fallback behavior are deterministic and independently testable. Symthaea can forecast or recommend; it cannot override constraints or become a single point of failure.
4. **Local capability is a first-class service.** Local DNS/service discovery, messaging, cached content, and community services should remain usable without WAN, external identity, or cloud telemetry where the declared profile requires it.
5. **Multi-path where it helps; store-and-forward where it must.** Use ordinary IP transports and multipath mechanisms for live paths. Use BPv7/DTN for disruption and scheduled-contact conditions. They solve different problems.
6. **Standards before invention.** Reuse 3GPP, IEEE 802.11, Passpoint, RADIUS/AAA, IETF, O-RAN, BPv7/BPSec, and CCSDS interfaces. Write proprietary extensions only after an interoperability gap is demonstrated.
7. **Trust is not consensus in the forwarding path.** Device authentication and authorization must be locally verifiable and work during partitions. Mycelix can support credentials, governance, accountability, and federation; Holochain consensus must not be needed to forward each packet.
8. **Evidence over aggregate scores.** Publish scenario inputs, exact source revisions, hardware, seeds, raw results, uncertainty, negative controls, and claim boundaries. A green CI badge is not a measured network result.
9. **Economics and access are engineering requirements.** Measure cost per active household and cost per successful useful task, including power, backhaul, installation, maintenance, replacement, volunteer labour, and donated assets.
10. **Graceful degradation, not hidden failure.** Unknown telemetry, stale authorization, clock uncertainty, full queues, loss of upstream, or unavailable control planes must have explicit states and bounded fallbacks.

## 3. Reference architecture

LNF is a set of cooperating planes. Their interfaces should be versioned and independently testable.

### 3.1 Endpoint and device plane

Smartphones, laptops, embedded devices, gateways, sensors, vehicles, and future spacecraft terminals use their native network interfaces. LNF initially integrates commodity hardware; custom RF or silicon is justified only by a measured limitation and a credible manufacturing/certification path.

Responsibilities: device identity and secure enrolment; network capability discovery; user-controlled privacy and cost preferences; application intent declaration; local queue/outbox for explicitly durable messages; observable link-state and power measurements.

An LNF agent must not be mandatory for basic IP connectivity or host boot.

### 3.2 Access and link-adapter plane

Adapters expose typed observations and capabilities for Ethernet, fibre-facing interfaces, Wi-Fi, Bluetooth/low-power personal networks, cellular data, authorized private LTE/5G, fixed wireless, satellite WAN, and local mesh.

A link reports facts such as state, measured capacity, loss, RTT, jitter, cost, energy, trust domain, route reachability, and observation age. Each field carries provenance, time, quality, and an explicit unknown state.

A physically up link is not proof that the Internet, DNS, an application, or a required destination is reachable. Health checks distinguish link, gateway, DNS, transport, and service reachability.

### 3.3 Connectivity broker

A pure Rust decision core takes application/user intent; eligible link candidates and observations; authorization and cost policy; network/site profile; and deterministic algorithm version.

It returns the selected route or explicit no-route result, hard-exclusion reasons, ranked alternatives, estimated trade-offs, input and policy digests, clock quality, and a proposed adapter action.

The core has no networking privileges, no device mutation, no radio control, and no external service dependency. A separate privileged adapter applies an approved plan and emits an execution receipt. Same inputs and version must produce identical decision bytes.

Hard exclusions precede ranking: unavailable path, prohibited route, stale/unknown required trust, cost/data cap exceeded, locality violation, insufficient capacity, unsupported transport, or revoked/expired authority. No weighted score may override a hard exclusion.

### 3.4 Live transport and multi-path plane

Use IP and mature transport protocols for live reachability. Preserve application sessions only where the chosen transport and both endpoints support it. Evaluate MPTCP (RFC 8684) and Multipath QUIC where implementation and deployment constraints are met; otherwise expose handover interruption and reconnect behavior honestly.

Separate three distinct decisions:
1. **Broker eligibility:** the provider-neutral broker (#64) determines which links may be used under trust, authorization, freshness, cost, data and quality policy.
2. **Path management:** the transport path manager creates/removes subflows and manages address/path lifecycle.
3. **Session scheduling:** a scheduler distributes bytes/packets among the already eligible live subflows, considering completion deadline, congestion, RTT/jitter/loss, data/cost/energy caps, shared failure domains and receiver reorder pressure.

LNF-MPATH-001 (draft PR #211) starts a deterministic Rust model and benchmark contract for the third decision. It does not modify MPTCP or QUIC wire formats, create subflows, or actuate routes. Linux MPTCP provides a real comparator with a separation between path manager and packet scheduler. Multipath QUIC -21 was IESG-approved as a Proposed Standard but remained in the RFC Editor queue as of 2 November 2026; its protocol specification does not define application scheduling. Do not call it a published RFC until publication is confirmed.

A multipath scheduler must not simply maximize the sum of advertised bandwidths. More paths can increase out-of-order delivery, head-of-line delay, receiver memory, energy and metered cost. Compare useful task completion and tail latency under the same quality/security/cost floor, and include single-path fallback where multipath is unavailable or does not improve the workload.

Do not tunnel every protocol through a single proprietary overlay. Avoid packet-content inspection. Distinguish same-LAN path change, Wi-Fi roam/reassociation, WAN route failover, cellular/eSIM profile change, multi-path aggregation within a session, application reconnect, and durable message forwarding. These are different events with different guarantees and metrics.

### 3.5 Local-first and DTN plane

Use BPv7 (RFC 9171) as the bundle protocol baseline for disruption-tolerant store-carry-forward. Evaluate BPSec (RFC 9172) and its default security contexts (RFC 9173) for bundle integrity and confidentiality. Compare implementations and interoperability before choosing a production dependency.

**Security boundary:** BPSec supplies bundle integrity and confidentiality services; RFC 9172 explicitly does not define hop-by-hop authentication and leaves key management to a separate network-management function. RFC 9173 provides default security contexts useful for interoperability testing, not a universal key lifecycle. LNF must separately specify security-source/acceptor policy, credential-to-principal mapping, key provisioning/rotation, revocation freshness during partitions, and what metadata remains visible. ION-DTN documents BPSec support; dtn7-rs's public project description establishes BPv7 support but is not evidence of BPSec support at any given revision. Pin and test both before selecting an adapter.

The LNF application contract separately defines stable message identity, expiry, traffic class, payload size limits, custody/receipt semantics, sender principal binding, duplicate disposition, and application idempotency. Do not equate daemon acceptance, bundle forwarding, endpoint delivery, transport-handler acceptance, and application commit.

**Exactly-once transport delivery is not a valid general claim.** A crash after an application side effect but before its durable receipt can cause redelivery. The reference consumer must use a durable inbox/idempotency transaction or document at-least-once semantics.

Use Xenia Wire for bounded interactive session protection only where its session-key lifetime and replay semantics match the workload. Do not treat a short live-session replay window as a durable DTN inbox. For DTN, assess BPSec and durable credential/revocation semantics. Any additional signed application envelope must justify why BPSec and existing identity standards are insufficient; avoid duplicating cryptographic machinery by default.

### 3.6 Identity, trust, and governance plane

Separate cryptographic device identity; network admission and authorization; application principal identity; route/provider eligibility; community membership and governance; and operational evidence and settlement.

Use established secure enrolment and access-control methods where appropriate (for example IEEE 802.1AR, BRSKI/RFC 8995, certificate lifecycle, 802.1X/EAP, RADIUS/AAA, and platform-specific SIM/eSIM credentials). Mycelix may represent federation membership, attestations, governance decisions, and accountability, but a disconnected gateway must verify a bounded local authorization policy without reaching a consensus service for every packet.

Revocation during a partition is fundamentally a freshness problem. The policy states maximum credential age, revocation-feed freshness, offline grace, clock requirements, and behavior when freshness is unknown. Never report a globally current revocation state when disconnected.

### 3.7 Operations, intelligence, and digital twin plane

- **Symthaea:** forecast congestion, demand, link quality, outage risk, and candidate resource allocations. Start in offline analysis and shadow mode. Recommendations include uncertainty and evidence references.
- **Symtropy:** provide mathematical models, network simulations, digital twins, topology and resource optimization, and fault-injection experiments. Integrate mature simulators instead of replacing RF/PHY simulators.
- **Mycelix:** device/federation credentials, attestations, governance, accountability, and optional economic settlement. Keep consensus and distributed ledgers off the packet fast path.
- **Sol-Atlas:** geospatial planning, terrain/coverage layers, fibre/backhaul routes, tower and relay placement, ground stations, service coverage, and asset lifecycle.
- **Luminous Platform / Luminous Edge:** deployment, configuration, safe operations, NixOS packaging, observability, receipt verification, and reference gateways.

All actuating control follows: observe -> propose -> validate against deterministic policy -> approve or meet a narrowly scoped pre-authorized policy -> apply -> independently observe -> verify outcome -> record receipt -> rollback or fail-safe on divergence. A model prediction is not proof that an action was safe or successful.

## 4. Standards and ecosystem adoption matrix

| Domain | Preferred baseline / candidate | LNF position |
|---|---|---|
| IP networking | IPv4/IPv6, DHCP, DNS, ICMP, routing standards | Interoperate; do not invent a parallel Internet |
| Wi-Fi | IEEE 802.11; Passpoint / Hotspot 2.0 | Integrate client and AAA behavior; measure actual roaming |
| Community mesh | 802.11s, batman-adv or other qualified routing | Topology options only; mesh does not create backhaul or encryption by itself |
| Cellular | 3GPP LTE/5G; future IMT-2030 | Use authorized commercial/private-network paths; no custom PHY initially |
| Open RAN | O-RAN interfaces and RIC architecture | Treat xApps/rApps as privileged attack surfaces; enforce zero-trust controls |
| Multipath transport | MPTCP, Multipath QUIC where supported | Benchmark per application and device; no universal seamlessness claim |
| Device enrolment | IEEE 802.1AR, BRSKI/RFC 8995, 802.1X, RADIUS/AAA | Reuse mature credential lifecycle; profile privacy and ownership transfer |
| DTN | BPv7/RFC 9171, TCPCLv4/RFC 9174 and suitable convergence layers | Interoperate with existing DTN implementations |
| DTN security | BPSec/RFC 9172 and default contexts/RFC 9173 | Prefer standards; separately test key management and resource abuse |
| Space links | CCSDS RF and optical communications standards | Implement adapters and interoperability profiles, not proprietary link assumptions |
| Device / fleet updates | TUF/Uptane-aligned signed update and rollback policy | Treat firmware provenance as part of network trust |
| Observability | OpenTelemetry-compatible metrics/traces where appropriate | Redact identities, credentials, payloads and browsing history from public receipts |

## 5. Benchmark and evidence contract

### 5.1 Evidence tiers

Every claim declares its evidence tier:
1. **Model/simulation** — assumptions, validation range, simulator and scenario revision.
2. **Hermetic software test** — deterministic inputs, fault injection, exact source SHA.
3. **Virtual/emulated network** — OS/kernel, emulator, topology, impairment configuration.
4. **Hardware-in-the-loop lab** — exact hardware, firmware, cabling, spectrum containment/authorization, calibrated instruments.
5. **Controlled field pilot** — site, sampling plan, weather/interference where relevant, participant consent, authorization, operating period.
6. **Production evidence** — sustained operating data, incident accounting, independent audit, support and service economics.

No higher-tier claim may be inferred from a lower-tier result. Queued, skipped, not run, missing artifact, or unmeasured is never PASS.

### 5.2 Core outcome metrics

| Area | Primary metrics | Required reporting |
|---|---|---|
| Access quality | useful task success, throughput/goodput, RTT/jitter/loss, coverage distribution | p50/p95/p99, cell-edge and worst-decile; workload and site |
| Handover/failover | interruption, reconvergence, new-flow restoration, in-flight loss | separate Wi-Fi roam, WAN failover, multipath and app reconnect |
| Connectivity broker | hard-policy violations, decision latency, determinism, explanation coverage | 0 policy violations; same-input byte-identical replay |
| Local-first | task success with WAN removed, local service availability | 24-hour outage test with power/LAN conditions stated |
| DTN | unexpired message delivery, latency, duplicate rate, expiry correctness, backlog age | delivery denominator, p50/p95/p99, route/contact trace |
| Durability | crash-window loss, replay behavior, custody/receipt state, fsync failures | crash at every state transition; no unsupported exactly-once claims |
| Security | unauthorized acceptance, replay/tamper rejection, credential freshness, key rotation | negative corpus and trust freshness boundary |
| Operations | detection time, diagnosis accuracy, safe recovery, rollback, change blast radius | false positives, false negatives, observer loss, exact action receipts |
| Energy | Wh per successful useful task/GB; idle and peak watts | meter and calibration, workload, radio state |
| Economics | fully burdened cost per active household and per successful task | capex, backhaul, energy, maintenance, replacements, labour and subsidy ledger |
| Equity/access | affordability, coverage by declared site/population strata, accessibility | predeclared sampling frame; avoid averaging away unserved groups |

Never use one weighted network score to conceal a hard failure. Publish a Pareto frontier for cost, energy, latency, reliability, coverage and privacy, with the task-quality floor held constant.

### 5.3 Initial proposed gates (not measured results)

- 100% of connectivity decisions have an explanation, algorithm version, input digest and policy digest.
- 0 accepted hard-constraint violations in the complete negative-control corpus.
- 100% deterministic decision replay for fixed inputs, policy and version.
- Essential local-only service profile remains functional for 24 hours with all WANs disabled, assuming declared local power and LAN dependencies remain available.
- At least 99% of 100 scripted primary-WAN failure trials restore **new eligible flows** within 30 seconds on the named reference gateway; existing session continuity is measured separately.
- 100% of expired messages in the test corpus are not newly dispatched after expiry under the declared clock policy.
- 100% of duplicate reference-consumer message IDs produce no second application side effect when the consumer's durable idempotency transaction is enabled.
- At least 99% of unexpired synthetic DTN messages are delivered after the named route/contact schedule recovers, with p50/p95/p99 delay and full denominator published.
- Zero committed receive-journal records are silently discarded in the crash/restart matrix after the journal commit point.
- Zero plaintext payloads, keys, credentials, subscriber identifiers or browsing history in public operational receipts.
- Report measured Wh/task and fully burdened cost/task before any optimization or cost-advantage claim.

These are pilot qualification targets, not general service guarantees. Tighten them only after a baseline and measurement uncertainty are available.

### 5.4 Fault and adversarial corpus

Minimum scenario families:
- cable pull, radio link loss, AP restart, gateway restart, device sleep and battery exhaustion;
- link-up with upstream blackhole, gateway reachable but DNS broken, DNS works but application is unavailable;
- high RTT, jitter, asymmetry, random/burst loss, reordering, duplication and MTU mismatch;
- captive portal, invalid TLS, rogue AP, wrong AAA realm, expired/revoked credential and stale trust feed;
- cost cap, data cap, energy cap, forbidden route, unavailable capacity and stale telemetry;
- duplicate message ID, conflicting message ID, expired bundle, invalid signature, tampered payload and oversized envelope;
- full disk, read-only filesystem, fsync failure, process kill before/after each journal/receipt transition, machine reboot and clock rollback/skew;
- malicious relay, unauthorized next hop, route oscillation, partition/reconnection and contact-window miss;
- observer/telemetry loss, model drift, poisoned forecast, invalid recommendation and attempted actuation outside policy;
- cross-client/VLAN leakage, firmware rollback, unsigned update and compromised edge credential.

Every scenario needs a positive control, negative control, expected disposition, bounded resource profile, and machine-readable receipt.

## 6. SOTA comparators and fair comparison method

LNF compares against existing systems, not a straw-man baseline.

| Domain | Baselines to include | What LNF may claim only after evidence |
|---|---|---|
| Commodity multi-WAN | selected OpenWrt/mwan3 or equivalent router policy, ISP CPE, provider router | better cost/policy transparency, recovery or local continuity under matched links |
| Wi-Fi federation | native Passpoint/OpenRoaming-compatible client/AP/AAA path where available | improved interoperability or measured association/roam behavior |
| Private 5G / RAN | srsRAN Project, OpenAirInterface and commercial baseline where accessible | specific throughput, coverage, energy, interoperability or operational improvement |
| RAN simulation | 5G-LENA + ns-3; calibrated site-specific propagation where available | model-level result only until hardware validation |
| Wireless emulation | EMANE or an appropriate emulator/testbed | behavior under a named reproducible impairment model |
| DTN | NASA/JPL ION-DTN and dtn7-rs, both pinned to exact revisions | interoperability, resource bounds, custody semantics or scenario-specific efficiency |
| Space networks | CCSDS profiles and existing DTN convergence implementations | compliant adapter/interoperability, never a claim of replacing flight-proven systems without flight evidence |
| Operations | deterministic rule/policy baseline and existing controller behavior | lower error rate, recovery time or energy under equivalent inputs and guardrails |

Fairness rules:
- Match hardware, antennas, spectrum/channel, transmit power, bandwidth, topology, backhaul, application workload, and measurement instrument.
- Pin exact software, firmware, simulator, kernel and dependency revisions.
- Publish seeds and run count; repeat enough times to report confidence intervals and avoid selecting only favorable runs.
- Include warm-up, steady-state and failure/recovery intervals.
- Separate offered load from successfully delivered useful data.
- Report tail latency, worst-decile coverage and failure cases, not just mean throughput.
- Include baseline tuning effort and operational complexity.
- Preserve raw results, scripts, environment manifests and checksums.
- Compare under equal security and quality floors; do not disable baseline security to make LNF appear faster.

## 7. Research and implementation stack

Do not reimplement mature simulators or radio stacks inside Symtropy.

- **5G-LENA + ns-3:** 3GPP NR system-level simulation; the upstream compatibility table lists NR v5.1 with ns-3.48 (August 6, 2026), while v5.2 is under development. Pin both exact revisions and apply/review the release notes' recommended ns-3 channel/performance patches before publishing results. It is a model, not proof of live RAN performance.
- **IMT-2030 research envelope:** the ITU-R WP 5D draft technical performance requirements were completed in February 2026 and draft evaluation guidelines in June 2026; both were submitted for SG 5 approval in December 2026. Use the six IMT-2030 usage scenarios and draft methods to shape research, but do not call them final approved requirements, a product certification, or a real-world SLA.
- **Site-specific radio validation:** 3GPP statistical channel models and system-level simulation are useful for controlled comparisons, but may miss deterministic street-canyon blockage, corner diffraction and site geometry. Where coverage or beam-management conclusions depend on those effects, use a documented ray-tracing/measurement lane and validate it against site measurements before field extrapolation.
- **Site-specific propagation / ray tracing:** use a calibrated ray-tracing path where the site and model justify it; compare against statistical channel models rather than assuming one is universally superior.
- **srsRAN Project / OpenAirInterface:** candidate open RAN lab stacks; validate exact hardware, software release, core, UE and supported features.
- **EMANE:** candidate wireless emulation for controlled propagation, mobility and interference experiments.
- **ION-DTN and dtn7-rs:** external DTN comparators; pin revisions and test the same bundle, routing, security and failure scenarios.
- **BPv7 / BPSec:** normative interoperability baseline; test with independent implementations and committed test vectors.
- **O-RAN:** study RIC architecture and NIST's 2026 RIC security guidance before enabling third-party xApps/rApps or exposing E2 control.
- **Symtropy:** scenario orchestration, validated models, topology/cost/energy models, output normalization, and benchmark reproducibility.
- **Rust:** deterministic policy engine, gateway control plane, verifier, adapters, receipt schema, and test tooling.

## 8. Phased engineering plan

### Phase 0 — Freeze the architecture and claim boundaries
- Adopt this document as draft architecture.
- Map issues #61, #63, #65, #67 and #69 to their owning repositories and interfaces.
- Record protocol/standards dependencies and open questions.
- Do not add radio or hardware scope before the gateway profile is frozen.

### Phase 1 — Pure Rust connectivity decision core
- Implement typed intents, link observations, hard exclusions, deterministic ranking, reason codes and canonical decision receipts.
- Add golden test vectors, negative controls, property tests and deterministic replay.
- No socket, route, firewall, SIM, radio or OS mutation in this crate.
- Use the existing provider-neutral connectivity broker issue as the implementation owner.

### Phase 2 — Local-first edge reference node
- Integrate a commodity gateway and local services.
- Support multiple authorized WAN adapters as independent candidates.
- Establish LAN-only continuity, DNS/service health distinctions, cost caps, and WAN-loss behavior.
- Preserve local observability when cloud telemetry is unavailable.

### Phase 3 — DTN reliability and security
- Continue the existing kitsune2-transport-dtn work without treating pending draft PRs as merged or qualified.
- Close the destructive receive-to-journal loss window at the daemon/API durability boundary, not just inside the adapter.
- Freeze message identity, expiry, disposition and idempotency contracts with test vectors.
- Evaluate BPv7/BPSec interoperability and ION-DTN/dtn7-rs behavior.
- Qualify crash/fault behavior at every durable state transition.

### Phase 4 — Wi-Fi federation and private-network lab
- Use existing Passpoint/OpenRoaming and 3GPP-compatible components where participation and authorization exist.
- Build a contained, lawful lab before a field pilot.
- Use 5G-LENA/EMANE for model and emulation, and hardware-in-the-loop for physical evidence.

### Phase 5 — Non-terrestrial and interplanetary profiles
- Use time-dependent contact graphs and BPv7/DTN convergence layers.
- Model Earth, orbital relay, Moon and Mars contacts with directional/asymmetric data rates, storage budgets, clock uncertainty and missed contacts.
- Compare to ION-DTN and relevant CCSDS procedures.
- Keep this separate from the terrestrial gateway release until its own conformance gates pass.

## 9. Existing implementation inventory and integration order

LNF is an integration programme, not a reason to create parallel projects. The following active work already exists and should remain with its current owners:

| Existing work | Owner / reference | LNF integration |
|---|---|---|
| Provider-neutral connectivity policy core, strict profile/receipt validators and outcome-benchmark roadmap | `luminous-platform` PR #64; issue #63 | Phase 1 implementation owner. Do not create a second policy engine. Exact-head software checks are reported passed in PR #64; live OpenWrt/VM and physical gateway evidence remain outstanding. |
| Community gateway, multi-WAN and local-service reference design | `luminous-platform` PR #62 / issue #61 | Phase 2 hardware and deployment reference. Starlink is only an optional authorized Ethernet/IP WAN; no assumption of resale rights or South African service availability. |
| Passpoint/OpenRoaming federation | `luminous-platform` issue #65 | Separate Wi-Fi federation adapter; credential profile, AAA and federation participation are distinct from route selection. |
| Local-first outage continuity and bounded store-and-forward | `luminous-platform` issue #67 | Application-level service contract and bounded queues; BPv7/BPSec interoperability is a separate protocol gate. |
| Fully burdened connectivity economics | `luminous-platform` PR #70 / issue #69 | Cost-per-household/task ledgers, including donated equipment and volunteer labour; inputs must be dated and scope-matched. |
| Network-device and fabric transition continuity | `symthaea` issue #1100 | Infrastructure configuration transitions and behavioral verification: Batfish, KNE/containerlab, gNMI/NETCONF/gNOI/Redfish adapters. This is distinct from end-user WAN route choice. |
| Mesh reliability–airtime–energy Pareto harness | `symthaea` issue #8634 | Low-power/local mesh transport benchmark lane; do not generalize it to Wi-Fi roaming, cellular, or satellite without new PHY/link evidence. |
| Containerlab + FRR network-twin adapter | `symthaea` issue #4834 | Reusable isolated routing/fabric emulation lane, exact runtime identity, and differential verification. |
| Power-aware telecommunications resilience planning | `sol-atlas-leptos` issues #323 and #325 | Geospatial declared-topology, site-power, backhaul/common-cause and service-equity planning. It is not RF coverage prediction or packet simulation. |
| F0 reproducible federation topology and network-emulation run manifests | `mycelix` issues #3251 and #3255 | Cross-node identity, topology, fault schedule and evidence-run manifest. Reuse the fixture/manifest design; do not create another emulation format. |
| Time-ordered underground communications and egress trace replay | `symtropy` issue #2097 | Timed mission-state replay and staleness semantics, not a packet-level network simulator. Keep the distinction explicit. |

### Integration boundaries that must remain separate

- **Connectivity broker versus fabric continuity:** PR #64 chooses among eligible access paths from declared intents and link observations. Symthaea #1100 qualifies planned changes to network devices/fabrics. One is per-flow/service path selection; the other is infrastructure transition safety.
- **Geospatial service resilience versus RF planning:** Sol-Atlas #323/#325 reasons over declared sites, capacities, power, backhaul and service demand. It must not infer radio coverage from distance or draw a packet-level conclusion from a topology graph.
- **Mesh versus wide-area access:** Symthaea #8634 evaluates constrained mesh reliability/airtime/energy. Its channel model, radio profile and payload semantics must be declared before comparing it with Wi-Fi, cellular or satellite.
- **Federation topology versus transport behavior:** Mycelix #3251/#3255 describes nodes, links, faults and run identity. It does not prove that a concrete transport delivers bundles correctly.
- **Mission event replay versus network emulation:** Symtropy #2097 validates time-ordered coordination state and stale evidence. It does not model packet queues, airtime, PHY behavior or route convergence.
- **Control-plane configuration versus data-plane outcome:** Symthaea #1100 explicitly preserves the boundary that device configuration acceptance is not network behavior verification or execution authority.

### Immediate execution sequence

0. Use LNF-EDGE-DTN-001 / PR #208 as the cross-project end-to-end campaign owner; it binds the broker, gateway, local-first and DTN stages without duplicating their implementations.
1. Review the LNF architecture PR #207 and reconcile it with the owner issues listed above.
2. Finish the software-only provider-neutral broker contract in PR #64, including reviewed dependency lockfiles; its existing exact-head test pass is not live network evidence.
3. Qualify the offline multipath model/profile in PR #211, then use issue #212 to measure real MPTCP subflows in an isolated namespace lab with exact kernel, path-manager, scheduler and congestion-control pins.
4. Qualify shared capacity domains and tenant-first weighted max-min in PR #215; tenant fairness must be evaluated across a shared bottleneck, not inferred from per-flow or per-session fairness.
5. Use PR #216's read-only plan to generate and review the isolated fq_codel/TBF and CAKE campaign. Only a separate explicit-opt-in executor may create namespaces/install qdiscs, and it must bind cleanup to resources created by that run.
6. Apply PR #222's RFC 6356 best-path and do-no-harm evaluator to exact paired Linux MPTCP/TCP trials from issue #212. Do not accept a throughput gain if a companion TCP flow is materially starved.
6. Run the isolated OpenWrt/two-WAN fault matrix and local-only service campaign on named gateway revisions; keep VM, hardware-bench and field evidence distinct.
7. In parallel, use Symthaea #1100/#4834 to qualify a small redundant routed-fabric topology against Batfish and live FRR probes; do not give this adapter production actuation authority.
8. Use Sol-Atlas #323/#325 to model a named service topology with power and backhaul common-cause failures using the existing shared service-network evaluators.
9. Continue DTN durability work and BPSec-vs-envelope interoperability research in `kitsune2-transport-dtn` issue #10.
10. Only after these terrestrial paths have reproducible receipts should LNF open the broader NTN/space link conformance and contact-plan campaign.

## 9. Repository responsibility map

| Repository | Responsibility | Must not own |
|---|---|---|
| luminous-platform | cross-project architecture, profiles, operations and assurance | radio PHY or proprietary replacement for Internet standards |
| luminous-edge | deployable NixOS edge composition and host integration | mandatory boot dependency for connectivity |
| xenia-wire | bounded live-session wire protection and session crypto | durable DTN receipt/inbox semantics |
| xenia-peer | peer service and secure operational endpoints | direct autonomous configuration outside policy |
| kitsune2-transport-dtn | Kitsune2 transport adapter and DTN receive/delivery qualification | claiming full Holochain conductor transport without integration evidence |
| mycelix | identity/federation/governance/attestation | consensus on packet-forwarding fast path |
| symthaea | prediction, diagnosis and constrained optimization proposals | sole authority for routing safety or privileged actuation |
| symtropy | validated models, simulation and digital twins | replacing mature 5G/PHY/EM/RF engines without evidence |
| sol-atlas-leptos | geospatial planning and infrastructure coordination | treating modeled coverage as field-tested coverage |
| LNF multipath policy core (draft PR #211) | offline, deterministic session scheduling among broker-eligible paths | path discovery, subflow lifecycle, route mutation or transport wire changes |
| LNF MPTCP namespace lab (issue #212) | real kernel negotiation, subflow observation, path failure and scheduler measurements in isolated namespaces | physical RF, carrier behavior or production router mutation |
| LNF tenant fairness allocator (draft PR #215) | two-level weighted max-min allocation among tenant principals and then flows; shared-capacity-domain accounting | authenticating the tenant principal, packet queueing/AQM or replacing CAKE/fq_codel |
| LNF isolated qdisc planner (draft PR #216) | read-only namespace command planning for fq_codel/TBF and CAKE triple-isolate comparisons | executing privileged qdisc changes, claiming benchmark results or changing host networking |
| LNF MPTCP coexistence evaluator (draft PR #222) | paired best-path and shared-bottleneck do-no-harm evidence evaluation against TCP controls | implementing a congestion controller or claiming kernel/field performance from model outputs |

## 10. South African deployment gates

Before any community or commercial field deployment, assess the exact service, topology, equipment, spectrum, provider agreements, and ownership model. ICASA distinguishes licence categories and exemptions; equipment type approval is a separate requirement unless an applicable exemption exists. A private lab, a community network, resold Internet service, a private cellular network, and a public telecom service are not interchangeable legal categories.

No transmission outside authorized spectrum and equipment conditions. No provider resale or sharing without required contractual permission. Type approval, spectrum rights, ECNS/ECS status or exemption, privacy obligations, electrical safety, site permissions, and consumer obligations must each be separately reviewed for the exact pilot.

## 11. Open questions that require experiments

1. Which commodity gateway and supported radio configuration is the lowest-cost repeatable reference node?
2. Which live-path combination (native routing, MPTCP, Multipath QUIC, application reconnect) yields the best real outcome per workload and device?
3. Which local trust and revocation policy remains safe and usable during partitions of hours, days, and longer?
4. Does BPv7/BPSec interoperability across ION-DTN and dtn7-rs meet payload, custody, security and resource requirements?
5. What durable delivery contract can be demonstrated across daemon crash, adapter crash, storage failure, duplicate delivery and application-side-effect ambiguity?
6. What is the lowest fully burdened cost per active household at selected South African site archetypes?
7. Which Symthaea recommendations improve measured outcomes beyond deterministic baselines after accounting for false alarms, telemetry loss and compute/energy overhead?
8. What must change for lunar/Mars contact graphs, and what can remain standard BPv7/CCSDS interoperability?

## 12. External references

- ITU IMT-2030 framework and 2026 draft technical performance requirements: https://www.itu.int/en/ITU-R/study-groups/rsg5/rwp5d/imt-2030/pages/default.aspx
- 5G-LENA NR module and release compatibility: https://cttc-lena.gitlab.io/nr/html/
- NIST 2026 site-specific ray-tracing-driven ns-3 research: https://www.nist.gov/publications/enabling-site-specific-cellular-network-simulation-through-ray-tracing-driven-ns-3
- srsRAN Project documentation: https://docs.srsran.com/projects/project/en/latest/
- EMANE documentation: https://emane.io/introduction
- NASA/JPL ION-DTN documentation: https://ion-dtn.readthedocs.io/en/latest/
- dtn7-rs implementation: https://github.com/dtn7/dtn7-rs
- RFC 9171 — Bundle Protocol v7: https://www.rfc-editor.org/rfc/rfc9171.html
- RFC 9172 — Bundle Protocol Security: https://www.rfc-editor.org/rfc/rfc9172.html
- RFC 9173 — Default BPSec Security Contexts: https://www.rfc-editor.org/rfc/rfc9173.html
- RFC 9174 — TCP Convergence Layer Protocol v4: https://www.rfc-editor.org/rfc/rfc9174.html
- RFC 8995 — BRSKI: https://www.rfc-editor.org/rfc/rfc8995.html
- O-RAN Software Community architecture overview: https://docs.o-ran-sc.org/en/f-release/architecture/architecture.html
- NISTIR 8625 (September 2026), O-RAN Near-RT RIC security: https://doi.org/10.6028/NIST.IR.8625
- ICASA services licensing: https://www.icasa.org.za/pages/services-licencing
- ICASA equipment type approval: https://www.icasa.org.za/pages/type-approval

## 13. Decision requested from reviewers

Reviewers should focus on whether the plane boundaries, standards choices, security semantics, benchmark denominators, evidence tiers, and phased implementation order are sound. Disagreement is expected and should result in a tighter design, not weaker claim boundaries.

**Non-claim:** This document is an architecture and benchmark proposal. It does not prove that LNF outperforms a commercial operator, constitutes a 6G implementation, provides universal coverage, or is ready for production or field deployment.
