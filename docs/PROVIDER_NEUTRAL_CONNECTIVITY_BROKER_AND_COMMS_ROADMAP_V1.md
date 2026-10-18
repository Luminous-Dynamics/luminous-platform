# Provider-Neutral Connectivity Broker and Communications Roadmap v1

**Status:** Research and prototype specification; no carrier agreement, handset integration, commercial service or field qualification is claimed.  
**Reviewed:** 2026-10-18  
**Issue:** [#63](https://github.com/Luminous-Dynamics/luminous-platform/issues/63)  
**Related:** [#61 community network hardware/WAN](https://github.com/Luminous-Dynamics/luminous-platform/issues/61), [#62 open community connectivity reference design](https://github.com/Luminous-Dynamics/luminous-platform/pull/62).

## 1. Product thesis

The working product concept is a **provider-neutral connectivity broker** (working title; not a carrier or MVNO yet). It should improve the outcomes users value in Google Fi—reliable connection selection, predictable pricing, and lower friction—while extending the model to community Wi-Fi, local-only connectivity, multi-WAN gateways, store-and-forward, and optional satellite paths.

Do not define success as "we have more networks" or "our algorithm picked Wi-Fi." Define it as:

- the user's stated quality needs are satisfied;
- hard spend/data/trust constraints are never violated;
- the choice can be explained and replayed;
- the system does not inspect communications content;
- local communication keeps working where possible during an upstream outage;
- the measured total cost to complete useful tasks falls against a matched baseline.

This is a software-and-network integration strategy first. A software layer cannot summon carrier access, roaming privileges, an eSIM profile, a public telephone number, licensed spectrum, or satellite service without the relevant operator and legal arrangements.

## 2. Honest benchmark: Google Fi

Google Fi is a strong incumbent to benchmark, not a straw man. Its current official pages describe:
- automatic cellular-network selection on supported phones and plans;
- service in 200+ international destinations for eligible plans;
- the Unlimited Premium plan at USD 65/month for one line, with plan-specific high-speed data and tethering limits;
- faster, expanded international coverage features announced in June 2026, with multiple international-network switching on supported Pixel devices.

Its constraints are relevant too:
- the multi-network experience depends on supported device/plan/country and isn't exposed as unrestricted manual selection;
- a second active SIM can prevent the documented Pixel international network-switching experience;
- prolonged international use can cause international data to be suspended until substantial use in the US is observed again;
- it is a US-centric mobile service, not a universal subscription freely issued to any resident of any country.

The differentiation hypothesis is **user control, local affordability, continuity without WAN, provider portability and community ownership**—not claiming better global cellular coverage before comparable carrier agreements are secured.

### Comparison dimensions to measure

| Dimension | Google Fi baseline | Luminous target |
|---|---|---|
| Cellular coverage | Operator and roaming partner footprint | Equal coverage only where the same access/partner rights actually exist; report country/operator gaps |
| Wi-Fi and community network | Wi-Fi calling and selected auto-connect features | Explicitly integrate owned/community/partner Wi-Fi with consent and trust constraints |
| Cost transparency | Published plan and international data limits | User-set hard spend cap, quote freshness, unit/currency clarity, cost per useful task and no silent paid-link fallback |
| Network choice | Operator-controlled selection on supported devices | Inspectable deterministic policy, user-declared priorities, decision reasons and offline replay |
| Extended residence abroad | Subject to Fi's policy and plan conditions | Use locally available authorized providers and plans through regional partners; no one-SIM-must-stay-home assumption |
| Offline function | Mobile service depends on a usable access path; some phone functions remain local | Designed local services/store-and-forward when all upstream links fail |
| Governance | Carrier/plan terms | Local cooperative/partner options and portable configuration; no claim that the cooperative can waive provider terms |
| Hardware/software openness | Carrier/OS device compatibility model | Open gateway/control-plane components where practical, with explicit closed-component disclosures |

## 3. Architecture boundary

Use three independently testable layers.

### A. Decision core — first implementation

A pure Rust library receives:
- the connection observations supplied by a platform adapter;
- a declared traffic intent such as realtime, messaging, browsing, background sync, bulk transfer, local-only or sensitive;
- an explicit policy containing hard cost, latency, reliability, capacity, trust, metering and telemetry-age limits.

It returns:
- eligibility or exclusion for every candidate;
- a stable selected path (or an explicit no-eligible-path result);
- a list of machine-readable exclusion/ranking reasons;
- ranked alternatives;
- an input/profile revision and stable digest at the adapter boundary.

The core never touches network devices, changes routes, chooses SIM profiles, activates eSIMs, sends radio traffic, inspects packets, infers intent from URLs/message contents, or invokes an AI model. An adapter can later translate an accepted decision into a platform-specific action behind explicit operating-system permissions and a separate authorization gate.

**Hard filters precede ranking.** Cost ceilings, authorization, data caps, trust requirements, minimum quality and observation freshness must not be tradeable for a high weighted score. If no candidate is eligible, fail explicitly and explain why; do not silently choose an expensive or untrusted route.

### B. Adapters — provider and platform integrations

- **Gateway:** OpenWrt policy routing and multi-WAN capabilities are the first integration surface. Its implementation must be pinned, compatible with the selected OpenWrt release and tested against actual link failures. Per-flow multi-WAN load-balancing is not the same as combining multiple links into one connection.
- **Wi-Fi roaming:** evaluate Passpoint/WBA OpenRoaming and local captive/onboarding flows. Only join federations where trust, credential policy, accounting and terms are established.
- **Cellular plans:** initially catalogue operator/eSIM plan metadata and provide user-consented recommendations; use official profile provisioning and carrier APIs only under written agreements and technical entitlements.
- **Satellite WAN:** model Starlink and other terminals as replaceable upstream adapters; exact kit, plan, geography, service terms and power/ethernet path remain separate evidence.
- **Mobile UI:** first expose current-link visibility, spending/quality policy, and recommendations. Do not promise arbitrary live SIM switching: the app's actual powers depend on OS, modem, device privilege, active-profile and operator constraints.
- **Private LTE/5G:** use Open5GS, srsRAN or Magma only for appropriate lab/private-network work and deployments with an authorized spectrum and core/network profile. Open-source core/RAN code is not spectrum permission or an operator license.

### C. Commercial service layer — only after the technical core

A genuinely Google-Fi-like retail offer requires wholesale agreements, roaming contracts, billing/tax/refund processes, support, SIM/eSIM profile entitlement and distribution, lawful identity/numbering handling where voice service is offered, and country-by-country regulatory review. This is a separate venture decision; it must not be hidden inside the router project.

In South Africa, ICASA's published services-licensing guidance says reselling electronic communication services obtained from upstream providers requires the applicable ECS licence exemption process. Exact current rules, conditions and any legal changes must be re-verified with ICASA and local counsel before a public offer. Equipment type approval and radio-spectrum authorizations are separate issues.

## 4. Paradigm shifts

1. **Intent before interface.** The user declares an intent and bounds (e.g. "voice-quality connection, no more than X cents/GB, authenticated network only"). The broker selects among eligible paths; it does not need to inspect content to guess what a user is doing.
2. **Contracts before scores.** Security, authorization, privacy, cost caps and required quality are hard invariants. Optimization begins only after they pass.
3. **Connectivity as a vector.** Coverage, cost, latency, throughput, reliability, energy, privacy/trust and offline utility are separate metrics. Publish Pareto trade-offs; don't hide a weak privacy or price result inside one scalar score.
4. **Local-first, not just internet-first.** Services are useful on a LAN/community mesh during backhaul loss. Queue permitted outbound data, synchronize when a route returns, and distinguish local availability from global Internet reachability.
5. **Provider portability.** Use standards-based IP and documented adapter contracts. Keep configuration, credentials where exportable, contact identity and local data portable; never promise that a telephone number, IMSI/eSIM, vendor plan or proprietary service is automatically portable.
6. **No surveillance needed.** Use aggregate counters, link quality, latency probes and user-declared policy. Avoid payload inspection, URL-history tracking, advertising profiles and cross-network movement histories.
7. **Open by evidence, not label.** Qualify the gateway, radio, firmware, BOM, licenses and recovery path individually. "Open gateway + proprietary satellite terminal" is a hybrid; document it honestly.
8. **AI is an optimizer, never the judge.** Symthaea can suggest cache placement or forecast congestion after baseline proof. Deterministic code enforces the actual caps, authorization and firewall policies.
9. **No-path is a real state.** When every path violates the user's hard constraints, say so. Do not silently buy roaming data, disable VPN, use an untrusted hotspot, or enable an eSIM to make the status icon look connected.

## 5. Communications verticals beyond broadband/mobile data

Prioritize by feasibility and leverage:

| Vertical | Opportunity | Mature building blocks | Important boundary |
|---|---|---|---|
| Home/community Wi-Fi roaming | One credential across participating schools, clinics, libraries, apartments and cafés | Passpoint; WBA OpenRoaming; RADIUS/certificate federation | Federation acceptance, credential trust and accounting need operators/partners |
| Multi-WAN gateway | Route a household/community over fibre, fixed wireless, cellular and satellite | OpenWrt, Linux policy routing, WireGuard where appropriate | Failover does not guarantee session preservation or bandwidth bonding |
| Travel connectivity/eSIM | Compare authorized local and roaming plans, track true cost, avoid surprise bills | GSMA consumer eSIM SGP.22 and provider APIs under contract | Provisioning needs device profile, provider entitlement and country/legal permissions |
| Voice and video | Lower cost and maintain quality under poor links; switch future calls to viable links | SIP, WebRTC, mature encrypted calling applications, adaptive codecs | Emergency calling/numbering, telecom interoperability and end-to-end identity require separate work |
| Messaging during outage | Queue encrypted, non-urgent messages and synchronize opportunistically | Store-and-forward patterns; IETF Bundle Protocol for disrupted networks; federated messaging where suitable | Delivery can be delayed; don't misrepresent it as real-time emergency telephony |
| Emergency coordination | Local alerts, check-ins, resource boards and authorized relays during outages | Local LAN services, battery-backed gateways, authenticated broadcast/acknowledgement | Life-safety use needs threat analysis, authority integration, accessibility and drills; never promise delivery without a path |
| Direct-to-phone satellite | SMS/SOS fallback on specific devices/services and authorized territories | Carrier/satellite partnerships and device-native implementations | Coverage varies by device, country, provider and service; an app cannot add missing RF support |
| Private LTE/5G | Campus, mine, farm, clinic, port or remote utility network where authorized | Open5GS core; srsRAN Open RAN; Magma federation/access network software | Licensed spectrum, lawful deployment, RF engineering, secure SIM/eSIM provisioning and operational support |
| Low-power IoT | Water, power, agriculture, environmental and asset telemetry | LoRaWAN and appropriate licensed/unlicensed sensor technologies | Low bitrate/duty cycle and regional band rules; it is not cheap general-purpose phone data |
| Public warning/broadcast | Deliver small messages to many devices, with receipt and correction tracking | Cell broadcast where available/authorized; SMS/USSD/email/app/web as complementary channels | Only authorized systems may send official cell-broadcast alerts; multi-channel doesn't guarantee reception |
| Inter-site/private communication | Secure voice, text, files and incident coordination for community organizations | WireGuard, SIP/WebRTC, established end-to-end encrypted protocols | Identity/recovery, directory, key changes and abuse handling are hard parts, not optional details |
| Contact and identity portability | Less lock-in to one phone number or messaging provider | Portable identifiers, contact export and federated services | Number portability, verification services and spam/abuse reputation remain provider and regulatory concerns |

For low-bandwidth settings, optimize usefulness per byte: text and voice first, adaptive video, resumable transfers, deduplicated updates, and offline application modes. Avoid claiming any one radio or protocol is best across every terrain, load, device and climate.

## 6. Benchmark program

All targets below are proposed. The first tests should use synthetic link observations and deterministic replay. Network and device tests come only after the policy API is stable.

| Benchmark | Initial target | Evidence method |
|---|---|---|
| Hard-constraint safety | 100% of ineligible candidates excluded in committed positive/negative corpus; zero silent cap/trust/authorization violations | Mutation tests for caps, stale observations, unknown prices, wrong currency, missing internet, untrusted path, no local peer, insufficient remaining bytes |
| Explainability | 100% of decisions include selection, ordered alternatives, reasons and a no-eligible-path result when applicable | Schema / API contract tests |
| Determinism | Identical policy + input + algorithm revision gives byte-identical canonical report | Fixed fixtures, stable sort/tie-break and digest tests |
| Privacy minimization | 0 decisions require packet content, message content, contact list or browsing history | Input schema/code review and sentinel-field negative control |
| Cost performance | ≥30% lower measured spend per useful task vs matched baseline while preserving identical minimum quality/security requirements | Workload manifest, bill/usage records, denominator for completed useful tasks |
| Quality | Meet declared p95 latency, throughput and reliability bounds for each traffic intent or return no route | Repeated test windows and failure-injection corpus |
| Router failover | ≥99% of 100 primary-WAN failure trials restore new eligible sessions within 30 seconds in the selected reference profile | Link loss, link-up/no-upstream, DNS failure, recovery and power cycle; existing sessions may reset |
| Wi-Fi roaming | Measure time from authenticated venue arrival to usable IP connectivity, failed authentications, privacy and cost; set target after establishing compatible device/federation baseline | Multi-site tests with explicit federation accounts, varied Android/iOS devices, lost RADIUS/DNS/upstream cases |
| Local continuity | Essential local services remain usable for 24 hours with all WAN links disconnected when site power is available | Physical unplug/blackhole test, not only simulation |
| Voice/video resilience | Establish baseline call setup success, p95 call setup delay, interruption duration, packet loss, MOS where valid, and route-cost per minute; target a 30% reduction in outage interruption before claiming improvement | Controlled cross-WAN call tests, endpoint/codec fixed, separate new-call failover from mid-call continuity |
| Store-and-forward | ≥99% of unexpired queued test messages are eventually delivered in the test corpus after an available path returns; zero duplicate logical delivery in idempotent fixture | Partition/reconnect/reboot tests; explicitly measure p50/p95 delay |
| Security boundaries | 0 client-to-client, guest-to-management, WAN-to-management unauthorized flows accepted in negative corpus; invalid config/updates rejected | Firewall and update tests on exact gateway profile |
| Energy | Measure Wh/day per served household and per delivered useful GB/task; publish idle/typical/peak and backup duration | Calibrated whole-site measurement including WAN terminal, conversion loss and battery |
| Coverage | ≥95% of predeclared survey points meet a defined minimum access contract in peak period | Geolocated survey internally; publish privacy-safe aggregation and exclusions set before test |
| Cost transparency | 100% of recommendations record plan/price source, timestamp, expiry, currency, taxes/fees if known and unknowns | Provider quote adapter tests |
| Portability | Switch/removal of one WAN provider leaves the local network and local services working; export policy without carrier credentials | Adapter removal and config export/re-import test |
| Commercial claim | No claim of "better than Fi" until the same-country/device/usage matched benchmark compares real eligible plans and partner-backed coverage | Public comparison with dates, taxes, limits, device requirements, exclusions and contract status |

**Important:** a target that cannot be measured reproducibly is not a benchmark. Freeze workload, hardware, firmware, app versions, test duration and denominators before collecting results.

## 7. Implementation slices that do not wait on CI

### Slice 1 — pure decision core

Create a Rust crate with no networking privilege and no external dependencies. Include candidate/intent/policy types, hard filters, deterministic rankings, machine-readable exclusion reasons, stable tie-breaks, canonical digest inputs and mutation tests. This is useful work before SIM APIs or hardware are available.

### Slice 2 — offline replay/simulation

Add fixtures for two or more links, unknown price, stale observations, data cap reached, captive/no-internet Wi-Fi, high-latency satellite, unstable cellular, privacy-sensitive intents, local-only peers and no valid route. Compare deterministic policy vs simple policies (Wi-Fi first, cheapest first, strongest signal first); do not assume the new policy wins.

### Slice 3 — gateway integration

Map the decision output to supported OpenWrt multi-WAN policy routing. First support connection selection/failover and app/service classes that can actually be represented at the gateway. Do not claim per-app routing for end-to-end encrypted traffic unless there is an explicit device/application mark or tunnel adapter.

### Slice 4 — user-facing control

Build a small Leptos/Rust UI to show connection state, estimated cost, policy choice, and why a candidate is excluded. Provide user-defined caps and explicit override. Start read-only; only separately authorized adapters can enact route/profile changes.

### Slice 5 — provider catalogue and eSIM partnerships

Define a versioned provider plan schema with country, legal entity/operator, accessible networks, device constraints, price/currency/tax, fair-use, speed caps, hotspot, renewal, expiry, data priority, roaming and terms source. A price can expire. Do not let scraping or inference become a source of truth; prefer contracted provider feeds or verifiable published plan references.

### Slice 6 — broader communications

Run independent tracks for Passpoint Wi-Fi, store-and-forward messaging, robust voice/video across links, low-power IoT, and authorized private cellular. Prioritize local communications and gateway failover before deep carrier-core engineering.

## 8. South African execution approach

1. Start with authorized existing ISP and community Wi-Fi partners, not a new national radio network.
2. Ask ICASA or local counsel whether the exact planned activity is licence-exempt, class-licensed or individually licensed. Keep provider reselling, frequency use, radio equipment type approval, numbering and emergency voice as distinct questions.
3. Identify existing operators and MVNO/eSIM wholesale partners willing to provide written roaming/service terms.
4. Build a price/coverage catalogue for South Africa, but also create region adapters so the product can be used in other countries without treating SA regulatory assumptions as global.
5. Choose one bounded cohort and publish service quality, bill costs, device compatibility and what failed. Only later expand to international roaming as the commercial partner footprint becomes real.

## 9. Dependencies and factual sources

Checked 2026-10-18. Pricing, coverage, roaming, product availability and law can change. Revalidate before procurement or public claims.

- Google Fi current plan comparison: https://support.google.com/fi/answer/15939269
- Google Fi international use, countries, device constraints and extended-use policy: https://support.google.com/fi/answer/6157794?hl=en
- Google Fi June 2026 international-network switching update: https://blog.google/products-and-platforms/products/google-fi-wireless/international-travel-features-5g-coverage/
- WBA OpenRoaming model and Passpoint-based automatic roaming: https://wballiance.com/openroaming/how-it-works/
- GSMA consumer eSIM remote provisioning, SGP.22 v2.7 (published 27 April 2026): https://www.gsma.com/solutions-and-impact/technologies/esim/gsma_resources/sgp-22-v2-7/
- GSMA constrained/IoT eSIM architecture, SGP.32 v1.3: https://www.gsma.com/solutions-and-impact/technologies/esim/gsma_resources/sgp-32-v1-3/
- OpenWrt multi-WAN behavior and limits: https://openwrt.org/docs/guide-user/network/wan/multiwan/mwan3
- OpenWrt nftables multi-WAN/failover: https://openwrt.org/docs/guide-user/network/wan/multiwan/mwan3-nft
- Open5GS core network: https://open5gs.org/
- srsRAN Project open-source 5G CU/DU: https://docs.srsran.com/projects/project/en/latest/
- Magma open access-network platform: https://github.com/magma/magma
- IETF Bundle Protocol v7 for delay-tolerant networking (RFC 9171): https://www.rfc-editor.org/rfc/rfc9171.html
- ICASA licensing and service-resale guidance: https://www.icasa.org.za/pages/services-licencing
- ICASA equipment type-approval and licensing overview: https://www.icasa.org.za/pages/licensing
- Google Pixel Satellite SOS supported devices/regions (not a general commercial data path; South Africa is not on the current listed availability table): https://support.google.com/pixelphone/answer/15254448?hl=en

## 10. Explicit non-claims

This RFC does not claim:
- that a software broker can access any carrier's radio/core network without permission;
- that global eSIM profile provisioning is open or universally available;
- that the user may arbitrarily switch SIM profiles on any phone;
- that Wi-Fi, mesh, LoRaWAN, satellite SOS or private LTE is a substitute for universal broadband;
- that the system beats Google Fi today on coverage, price or service quality;
- that simulated tests establish field coverage, privacy security or commercial legality.

The first valuable deliverable is a reproducible decision core with negative tests. A provider-facing commercial product is a later milestone gated by real partnerships, region-specific legality, actual prices and physical/device interoperability evidence.
