# Open Community Connectivity Reference Design v1

**Status:** Research RFC; architecture and proposed test targets only. No field deployment, physical hardware qualification, vendor authorization, or Starlink South African service authorization is claimed.  
**Reviewed:** 2026-10-18  
**Issue:** [#61](https://github.com/Luminous-Dynamics/luminous-platform/issues/61)  
**Related work:** [#58 community hardware qualification](https://github.com/Luminous-Dynamics/luminous-platform/pull/58), [#60 offline Rust receipt verifier](https://github.com/Luminous-Dynamics/luminous-platform/pull/60), [#19 product line and open-hardware qualification](https://github.com/Luminous-Dynamics/luminous-platform/issues/19).

## 1. Decision

Build a standards-based, modular community network by **pooling legal upstream connectivity, distributing it efficiently, keeping eligible services local, and publishing repeatable hardware/software evidence**. Reuse supported hardware first. Develop custom hardware only for a demonstrated cost, repairability, energy, security, or supply-chain gap.

The initial design must support a Starlink terminal as an optional WAN uplink through the connection mechanisms supported for its exact kit and service plan. Starlink is neither an open-hardware component nor a required dependency of the network. Fibre, fixed wireless, cellular, other authorized satellite providers, or no WAN at all must remain valid operating states.

The first prototype is not a new radio, satellite modem, cellular base station, or proprietary mesh. It is a reproducible **community gateway + access/relay + local-services + power** system.

## 2. Paradigm shifts

### 2.1 Optimize for useful connectivity, not a headline speed test

Track the full cost to deliver useful local and internet services to active households. A gigabyte served from a legitimate local cache, a successful offline task, and a gigabyte retrieved through expensive satellite backhaul have different economics. Report those separately instead of hiding them in one traffic number.

### 2.2 Separate local availability from upstream availability

A gateway can provide DHCP/DNS, local documentation, cached public resources, community notices, and permitted local communication while every WAN link is down. This is not "the global internet offline"; it is a service contract that remains useful during an upstream outage.

### 2.3 Treat WAN technologies as replaceable interfaces

A WAN adapter exposes observed Ethernet/link state, address acquisition, reachability probes, MTU/path measurements, metered consumption when available, and documented service constraints. It does not require a vendor cloud API to pass ordinary packets.

The initial target is **policy-based failover**, not magical aggregation. When the primary link fails, new connections should move to a healthy secondary link. Existing sessions may reset because NAT mappings, public addresses, carrier-grade NAT, routing, and transport sessions can change. Combining the capacity of separate links requires a separate, explicitly tested multipath/tunnel design and may need an external endpoint.

### 2.4 Openness is a property of artifacts, not a product sticker

For every component separately record:
- editable schematic/PCB/CAD/RTL sources and immutable source revision;
- per-artifact license and redistributability;
- complete/partial BOM, exact MPNs, approved substitutes, and sourcing evidence;
- manufacturing, assembly, programming and test outputs;
- boot firmware, radio firmware, binary blobs, update paths and recovery;
- relevant type approvals, spectrum permissions, electrical/environmental tests, and physical qualification.

"Source available" does not necessarily mean OSHWA-compliant open-source hardware, reproducibly manufacturable, or fully open from boot ROM through radio firmware. The qualification/evidence contract under PR #58 remains the program boundary; this document does not introduce a competing maturity scale.

### 2.5 Make intelligence advisory and forwarding deterministic

Symthaea can be evaluated for cache placement, demand prediction, congestion forecasts and energy-aware recommendations only after a measurable baseline exists. The essential packet path, firewall, local services, safe fallback and recovery must work without Symthaea, Mycelix, a cloud controller or an external AI API.

## 3. Reference topology

```text
   WAN sources (one or more; each independently optional)
   ┌────────┬─────────────┬──────────┬───────────────┐
   │ Fibre  │ Fixed radio│ Cellular │ Satellite     │
   │ ISP    │ / WISP     │ modem    │ e.g. Starlink │
   └───┬────┴──────┬──────┴────┬─────┴──────┬────────┘
       └───────────┴───────────┴────────────┘
                           │ Ethernet/IP WAN
                ┌──────────▼───────────┐
                │ Open gateway         │
                │ DHCP; firewall;      │
                │ VLANs; WAN health;   │
                │ policy/failover; QoS │
                └─────┬─────────┬──────┘
                      │         │
           ┌──────────▼──┐   ┌──▼────────────────┐
           │ Wired / PtP │   │ Local services    │
           │ backhaul    │   │ cache; docs; DNS; │
           └──────┬──────┘   │ store-and-forward │
                  │          └───────────────────┘
        ┌─────────▼───────────┐
        │ Outdoor AP / relay  │
        │ client isolation;   │
        │ measured RF profile │
        └─────────┬───────────┘
                  │
        Homes / schools / clinics / community sites

   Separate trust zones: client | local service | WAN | management
   Separate power and physical service plan for each site
```

**Topology rule:** prefer fibre or a well-engineered point-to-point link when its lifecycle cost, capacity, line of sight, and repair path beat multi-hop mesh. Use mesh where a measured obstacle, mobility, route diversity, or deployment-cost benefit justifies it. Every wireless hop consumes airtime and adds contention; do not assume a larger number of relays increases end-to-end capacity.

Use interoperable IP networking and established protocols rather than a Luminous-only packet format. IETF Babel (RFC 8966) is a candidate for dynamic routing in appropriate wired/wireless mesh topologies, subject to a pinned implementation and experiment. It is not mandatory for a simple hub-and-spoke first pilot.

## 4. Hardware plan: qualify modules before designing boards

### 4.1 Gateway

**Initial software baseline:** OpenWrt on an exact supported device. Evaluate OpenWrt One as a candidate because the OpenWrt project positions it as a router designed around software freedom and repairability. Do not mark it qualified until the exact board revision, artifact licenses, BOM, supply, firmware, recovery, throughput and thermal/power results have been inspected.

The first gateway should provide separate physical/logical interfaces for WAN, community clients, local services and management. Apply default-deny inter-zone rules, client isolation where supported, least-privilege management, reproducible configuration and a documented factory/recovery procedure.

### 4.2 Outdoor access and relay

Start with an exact, supported, type-appropriate outdoor AP/radio model or a documented OpenWiFi-compatible platform where its hardware, software release, radio firmware, and operator tooling are actually available. Do not assume every OpenWrt-supported router is weather-rated, suitable for tower use, able to power another device, or legal with every antenna.

Prefer standards-based Ethernet and documented 802.3 PoE where compatible. Power and Ethernet over a cable must not be conflated: verify the device's supported PoE mode/voltage and load before connecting it.

### 4.3 Local-services node

Use an independently replaceable low-power computer with storage sufficient for the chosen workload. The initial scope is caching only content whose licence/access policy allows it, locally hosted public information, content-addressed update distribution, basic network diagnostics, and queued messaging or synchronisation that tolerates outages.

Cache policies must specify freshness, invalidation, privacy, access control and deletion. Do not mirror paywalled, personal, or otherwise restricted content just because it is technically cacheable. Do not inspect message bodies or browsing content to optimize traffic.

### 4.4 Power, enclosure and mounting

Publish editable enclosure/mounting files, cable schedule, grounding/lightning-protection design notes, service-clearance requirements, and a DC-power block diagram. Use a conservatively engineered power path with documented protection, measured loads, and separately qualified battery/charging components. Do not invent a custom lithium battery management system or mains converter as the first milestone.

Measure energy at the complete site boundary, including radio, gateway, backhaul terminal, cache, conversion losses, battery/solar losses where applicable, and standby load. A "low-power router" does not make a Starlink-connected site low-power by itself.

### 4.5 When first-party hardware is justified

Only open a custom-board design task after a source-reviewed comparison shows a named gap: e.g. excessive idle power, no repairable suitable enclosure, unavailable supported radio hardware, missing DC input, or an unacceptably high per-site cost. Before layout, specify RF regulatory path, mechanical/weather envelope, component lifecycle, factory/test fixtures, firmware supply chain, manufacturing yield, spares and long-term maintainership.

The complete network cannot honestly be called fully open while its chosen satellite terminal, radio SoC/firmware, or other mandatory component is closed. The achievable goal is modular openness with no proprietary component controlling the entire downstream LAN or storing its only configuration.

## 5. Starlink compatibility contract

### 5.1 Integration boundary

Treat Starlink as a vendor-supplied terminal and upstream service. Connect the exact terminal/kit using its supported Ethernet path and any required manufacturer adapter/cable. Configure the Luminous/OpenWrt WAN interface for DHCP where supported. Use bypass mode only when the specific kit and intended topology require it; the help documentation describes different connection steps across Standard Actuated, Standard, Performance and Mini kits.

Do not:
- open or alter the dish, alter RF components, inject unapproved power, or rely on undocumented connector pinouts;
- assume every Starlink kit exposes the same Ethernet connector, routing, power input or bypass behavior;
- scrape or require a private/vendor control API for normal WAN operation;
- promise public IPv4, inbound port reachability, a static public address, or preservation of sessions during failover;
- claim compatibility from a generic Ethernet link alone without running the profile-specific tests.

For Starlink Mini, follow the official cable/weather-seal guidance: using a third-party cable can change the device's stated water-ingress protection. Verify terminal-specific power requirements and measure actual draw rather than powering it from a generic PoE injector.

### 5.2 Required WAN profile behavior

The `starlink-ethernet-wan` profile is an adapter configuration, not a dependency in the routing protocol. It must record and test:

1. exact terminal family, hardware revision where identifiable, supported Ethernet adapter/cable and service-plan jurisdiction;
2. DHCP address acquisition, default route, DNS behavior, IPv4/IPv6 availability, MTU/path-MTU behavior and captive/blocked state;
3. reachability independent of DNS alone (Starlink's support guidance warns that its DNS servers do not respond to ping);
4. health-check interval, failure threshold, recovery hysteresis and primary/backup preference;
5. router bypass-mode state where applicable, client-LAN isolation, management access restrictions and recovery after cable/terminal/router power cycling;
6. observed idle, typical and peak input power, with instrument and measurement boundary recorded;
7. measured download/upload throughput, unloaded and loaded latency, loss/jitter, congestion times and outage behavior for the actual service location and plan.

Test link-present-but-no-internet and DNS failure separately. A cable status or a single ICMP endpoint cannot establish that Internet service is healthy.

### 5.3 Service terms and South African authorization are separate gates

A Starlink kit accepting Ethernet establishes a technical integration path; it does not establish that the particular plan permits resale, community hotspot provision, or distribution to third parties. Starlink terms vary by jurisdiction and plan. At least some published consumer terms prohibit resale except when authorized and describe community Wi-Fi/hotspot resale for Priority plans. Before offering shared paid service, confirm the actual local contract and written authorization rather than extrapolating from another country's terms.

South African deployment must separately verify current ICASA authorizations and applicable rules. ICASA distinguishes service licensing from radio-spectrum licensing and published draft satellite spectrum amendments on 15 May 2026, with submissions due 29 June 2026; those drafts are not an authorization for a specific operator or terminal. An ITWeb report dated 15 June 2026 quoted the Department of Communications and Digital Technologies saying ICASA had not received Starlink applications for spectrum, I-ECNS or I-ECS licences at the time of that parliamentary reply. This is the latest specific application-status report located in this review, not proof of today's status. Re-check ICASA and the provider before procurement/deployment. Until service authorization and plan permissions are evidenced, use an authorized local ISP for field trials and an ordinary Ethernet/DHCP uplink emulator for compatibility-development tests.

### 5.4 Required interoperability tests

- Starlink WAN direct into the gateway: address, route, DNS, MTU, health probes, measured path properties.
- Terminal plus gateway plus separate indoor/outdoor AP; confirm there is no double-NAT dependency in local service operation (double NAT may occur upstream, but is not grounds to expose the management plane).
- Starlink + legal backup WAN: unplug primary, blackhole upstream without dropping Ethernet, fail DNS, restore primary and flap power. Check route churn and new-session recovery.
- Confirm local DNS/cache/docs/messaging remain available for at least a 24-hour simulated upstream outage, assuming site power remains available.
- Confirm the system does not claim bandwidth aggregation when only failover has been implemented.
- Confirm disabling/removing Starlink leaves the same downstream network operational with another supported Ethernet WAN.

## 6. Software responsibilities across Luminous projects

| Component | Responsibility in this design | Must not become |
|---|---|---|
| Sol-Atlas | Site/terrain/demand layers, candidate gateway placement, coverage gaps and cost-to-reach analysis with source/confidence metadata | An authoritative RF propagation measurement without validated inputs |
| Symtropy | Discrete-event/site digital-twin experiments for capacity, queueing, outages, energy and topology choices | Proof that physical radios, thermals or RF coexistence pass |
| Symthaea | Advisory cache/link/power optimization evaluated against deterministic baselines | A required packet-forwarding dependency or sole decision-maker for safety/firewall rules |
| Mycelix | Optional community governance, shared-finance records and transparent allocation after integration tests | A source of network truth based only on reputation, votes or self-attestation |
| Luminous Platform | Pinned configurations, policy boundaries, monitoring, recovery, hardware receipts and repeatable deployment | A second router OS, unreviewed remote actuator, or substitute for physical evidence |

The first prototype can be useful without integrating all five projects. Establish plain IP connectivity, local services, telemetry and a reproducible configuration first; add advanced coordination only when it improves a measured outcome.

## 7. Benchmarks and proposed gates

Targets below are engineering hypotheses for a named basic-access pilot, not existing measurements, service guarantees or requirements applicable to every deployment. Publish workload, site, sampling window, device identity, tool revisions and raw artifact digests with each result.

| Dimension | Initial target | Measurement contract |
|---|---|---|
| Local-first utility | Essential local services operate for 24 hours with every WAN disconnected; zero per-request charge for those local services | Exercise clients with WAN cable physically removed and with upstream blackholed; separate local power/network failures |
| Coverage and user experience | At least 95% of predeclared survey points achieve at least 10 Mbit/s down and 2 Mbit/s up during the stated busy-hour test | Fixed device, height, orientation, test duration, client count, RF band/channel and retry/packet-loss record; publish failures and points excluded before testing |
| Local latency | p95 client-to-local-gateway RTT <= 20 ms for the declared Wi-Fi/Ethernet profile | At least 1,000 samples per point/load state; report wired baseline and radio retries separately |
| Loaded latency | <= 50 ms p95 added RTT at the constrained WAN under the published download/upload load where SQM CPU budget permits | Compare unloaded and loaded samples using the same endpoint; report throughput trade-off and router CPU utilization |
| Availability | Target 99.5% monthly service availability, with local-service, LAN, individual-WAN and global Internet availability reported separately | Exclude no outage silently; define planned maintenance policy, probe interval, measurement location and denominator |
| WAN failover | >= 99% of 100 scripted WAN-loss/recovery trials restore new outbound sessions within 30 seconds | Include cable loss, link-up/no-upstream, DNS failure and restored primary; existing sessions may reset; no pass from only link-state changes |
| Cache efficiency | >= 50% byte-hit ratio on a predeclared cache-eligible workload after warm-up; report whole-traffic byte reduction separately | Use rights-cleared reproducible fixtures, cold/warm runs, cache validation and freshness/invalidation tests |
| Energy | Publish Wh/day and W idle/typical/peak per entire site; demonstrate at least 8 hours of required local service on the selected backup-power profile | Calibrated meter; state battery usable capacity, conversion losses, temperature, service loads and reserve policy |
| Open-hardware traceability | 100% of critical hardware/firmware artifacts have source/availability/license status recorded; no unknown critical part silently treated as open | Immutable source/artifact hashes; exact MPN and revision; list blobs, missing fabrication files, license ambiguity and approved/unapproved substitutes |
| Repeatability | Two independently assembled/tested units for any design promoted beyond a one-off lab example, where logistics permit | Compare build instructions, BOM deviations, recovery steps, test outputs and unexplained variance; otherwise restrict the claim to the actual sample |
| Security boundaries | Zero accepted unauthorized client-to-client, guest-to-management or WAN-to-management flows in the negative corpus; all invalid/unauthorized update signatures rejected | Separate network, privilege, update and rollback tests; retain the negative cases, not just pass counts |
| Economics | Seek >= 30% lower fully burdened monthly cost per active household than the best feasible legal local alternative at the same declared service profile before claiming cost advantage | Include amortized hardware, transit/ISP subscription, energy, mounting/site cost, installation, support, replacements, repair, compliance, taxes and paid labor; report donated labor/equipment separately |
| Operations | Baseline and reduce technician-hours per 10 active nodes per month; publish median repair time and locally available spare parts | Include failed nodes, travel, failed updates and power-related incidents, not only healthy installations |

Use traffic shaping such as OpenWrt SQM/CAKE where supported to limit queueing delay, but measure actual throughput loss and CPU limitations. Target values must be revised from observed site demand, capacity and economic evidence; do not tune the metric to conceal a failed design.

## 8. Economic model and decision rule

For each topology, calculate:

```text
monthly_total_cost =
    amortized_capex
  + upstream_service_and_transit
  + electricity_and_power_storage
  + site_access_and_mounting
  + installation_and_paid_maintenance
  + support_and_replacement_spares
  + compliance_and_insurance
  + expected_failure_and_recovery_cost

cost_per_active_household = monthly_total_cost / mean_active_households
cost_per_useful_GB = monthly_total_cost / eligible_useful_GB_delivered
```

Report local-served bytes, upstream-served bytes, cache-eligible bytes, bytes avoided on the paid uplink and offline-completed tasks separately. Do not count donated hardware or labor as free; include an imputed-cost sensitivity case. Compare against the best feasible legal alternative with a matched geographic coverage and service level. If the design is not at least 30% cheaper, determine whether resilience, access, ownership, local availability or repairability still justifies the added cost, and state that trade-off explicitly.

### Funding model hypotheses

Test combinations, not ideology: anchor institution sponsorship, community-owned cooperative assets, bulk capacity purchases, public-service contracts, grants, limited premium tiers, and operator/ISP wholesale partnerships. Never make the essential tier depend on targeted surveillance, opaque token appreciation, indefinite unpaid operation, or an upstream resale arrangement that the provider has not authorized.

## 9. Execution plan that does not wait for CI

### Slice A — desk/manifest (no hardware required)

- Freeze the target service profile and the survey/load-test method before device selection.
- Select exact gateway/AP/edge/power candidates from currently supported products.
- Capture every candidate source URL/revision, artifact licenses, BOM status, firmware/blob state, price/quote date, repair route and South African type-approval/authorized-radio status.
- Derive an expected site energy budget from published specifications, label it estimated and replace it with calibrated measurements later.

### Slice B — offline software lab

- Create reproducible WAN emulation using ordinary Ethernet interfaces and controllable DHCP, DNS, packet loss, delay and upstream blackholes.
- Exercise firewall/VLAN isolation, local-only service, cache behavior, SQM, boot/restart, invalid config rejection and failover.
- Test a plain primary/backup routing setup before testing load balancing or multipath.
- Preserve all command output, tool and profile revisions, generated files, environment details and SHA-256 digests. VM/emulator results remain VM/emulator evidence.

### Slice C — physical bench

- Use an exact source-reviewed gateway and AP, calibrated power measurement, controlled client workloads and verified recovery path.
- Run 24-hour soak, heat/power-cycle tests, RF coverage and interference surveys, link-failure injection, firmware-update/rollback tests and independent reruns.
- Generate qualification receipts under the existing PR #58 / #60 contract; receipt validation is not a substitute for reviewing the truth and scope of evidence.

### Slice D — legally authorized pilot

- First deploy with a local ISP/fixed service whose terms authorize the intended sharing. Add Starlink only when local authorization, exact plan/kit, and sharing/resale permissions are documented.
- Run the pilot long enough to include real peak-hour demand and weather/power events.
- Publish cost and reliability against a fair baseline, redacting personal identifiers, access credentials, private topology and exact sensitive locations.

**Go/no-go:** no custom-board commitment until the integrated off-the-shelf reference is measured; no cost-saving claim until matched economics pass; no operational qualification until exact physical evidence passes; no shared Starlink field deployment without the relevant provider and regulatory permissions.

## 10. Sources and external dependencies

Sources reviewed 2026-10-18. Product specifications, licenses, plan terms, availability and regulations can change; re-check them at intake and before deployment.

- Starlink, *Can I use my own router with Starlink?* — kit-specific Ethernet/bypass paths and DHCP: https://starlink.com/fm/support/article/0b372c01-c692-8920-8396-ffb7f429aab1
- Starlink, *Starlink Mini WiFi Troubleshooting* — Mini Ethernet/cable and water-ingress guidance: https://starlink.com/za/support/article/acb3bee4-53d9-2f4f-ca45-c21db163543c
- Starlink, *Availability Map* — current local address and regulatory availability should be checked rather than inferred from global coverage: https://starlink.com/za/map
- Starlink legal terms portal — apply the actual jurisdiction/product-plan terms, not an unrelated country's template: https://starlink.com/legal
- ITWeb, *No Starlink licence application in SA yet*, reporting dated 8 October 2026: https://www.itweb.co.za/article/no-starlink-licence-application-in-sa-yet/5yONPvErdlP7XWrb
- ICASA, *Services Licensing*: https://www.icasa.org.za/pages/services-licencing
- ICASA, *Spectrum Licensing*: https://www.icasa.org.za/pages/spectrum-licensing
- ICASA, *updated National Radio Frequency Plan 2026*, published 24 July 2026: https://www.icasa.org.za/news/2026/icasa-publishes-an-updated-national-radio-frequency-plan-2026
- ICASA, *Draft Amendments to the Radio Frequency Spectrum Regulations and Radio Frequency Spectrum Fees Regulations*, published 15 May 2026: https://www.icasa.org.za/news/2026/icasa-publishes-draft-amendments-to-the-radio-frequency-spectrum-regulations-and-radio-frequency-spectrum-fees-regulations
- OpenWrt, *OpenWrt One*: https://openwrt.org/toh/openwrt/one
- OpenWrt, *Supported devices*: https://openwrt.org/supported_devices
- OpenWrt, *Smart Queue Management*: https://openwrt.org/docs/guide-user/network/traffic-shaping/sqm
- Telecom Infra Project, *OpenWiFi*: https://www.telecominfraproject.com/openlan
- IETF, *RFC 8966 — The Babel Routing Protocol*: https://www.rfc-editor.org/rfc/rfc8966
- Open Source Hardware Association, certification requirements: https://certification.oshwa.org/requirements.html

No external vendor claim is automatically an engineering result. All candidates, exact hardware revisions, source bundles, licensing, inventory, prices, radio authorizations and performance results remain to be independently verified for the intended deployment.
