# Connectivity outcome benchmarks and pilot protocol v1

**Status:** research methodology and proposed targets, not measured results.  
**Scope:** provider-neutral access brokerage, community Wi-Fi, multi-WAN gateways, local-first services and store-and-forward messaging.  
**Initial geography:** Johannesburg / South Africa, with all prices, regulatory status, equipment and coverage revalidated for a named deployment.

## 1. Design the outcome, not the radio

The top-level unit is a **successful useful task**, not an advertised megabit, a gigabyte purchased, a router uptime counter or a SIM selected. Example task classes should be independently declared before testing:

- complete an authenticated web form and receive its durable receipt;
- conduct a defined low-bandwidth voice call;
- exchange an encrypted message with a known recipient;
- access a cached public-service or learning resource while upstream is absent;
- upload a bounded work artefact and verify its remote digest;
- restore new eligible sessions after a WAN failure.

A task's quality, security, geographic scope and success condition must be matched between the candidate system and baseline. Count failures, timeouts, retries, support load and the people who could not use the service. Do not redefine success after seeing the result.

This framing is consistent with the ITU's Universal and Meaningful Connectivity framework: connection quality, availability, affordability, devices, skills, and security are all required dimensions; constant connectivity is not the only valid outcome. South Africa's reported 2025 ITU price for the standardized 5 GB data-only mobile-broadband basket is 1.27% of GNI per capita. That is useful for national comparison, not a household-level affordability guarantee or a sufficient usage allowance.

Sources:
- ITU, Universal and Meaningful Connectivity: https://www.itu.int/itu-d/sites/projectumc/aboutumc/
- ITU DataHub, South Africa 5 GB mobile broadband basket (2025): https://datahub.itu.int/data/?e=ZAF&i=36056
- ITU, 2025 ICT price statistics and basket methodology: https://www.itu.int/en/ITU-D/Statistics/Pages/ICTprices/default.aspx

## 2. Paradigm shifts to implement

### A. Connectivity becomes a portfolio of paths

Represent fibre, fixed wireless, authorized cellular, Wi-Fi federations, satellite WAN and local-only routes as separate adapters behind a common policy contract. Rank only candidates that satisfy hard rules for authorization, trust, telemetry freshness, user cost limit, data allowance and task requirements. Weighted preferences may rank eligible candidates; they must never buy their way past an exclusion.

OpenWrt's documented multi-WAN pattern commonly distributes separate IP connections rather than bonding multiple WANs into a single higher-speed connection. Measure what the chosen adapter actually does. Report failover, per-flow load balancing and any real multipath aggregation as different capabilities.

Reference: OpenWrt mwan3 documentation, including its explicit non-bonding caveat: https://openwrt.org/docs/guide-user/network/wan/multiwan/mwan3

### B. Internet interruption does not equal service interruption

Keep locally useful resources and local messaging on the LAN: service directory, emergency contacts/instructions, verified public documents, local education resources, status page and queued encrypted messages. Define which functions work without DNS or cloud identity providers; don't merely unplug the WAN and discover all the local services still depend on cloud calls.

The IETF Bundle Protocol defines a store-carry-forward overlay for challenged networks with intermittent connectivity, delay and high bit-error rates. It provides useful architecture precedent, but no protocol alone guarantees eventual delivery, end-to-end security or emergency-grade service.

Reference: RFC 9171, Bundle Protocol Version 7: https://www.rfc-editor.org/rfc/rfc9171.html

### C. “Free data” becomes an explicit funding and accounting contract

Global internet capacity is not free to operate. Use auditable combinations of pooled procurement, community cooperative shares, public-service contracts, schools/clinics/SME sponsorship, grants, municipal infrastructure, local caching and capped hardship access. Local content can incur zero marginal upstream data cost while still having real hardware, power, labour and maintenance costs.

Publish separately: cash paid, donated equipment/time, subsidies, unpaid labour, service revenue, debt, outage loss and estimated replacement cost. Never report donated or unpaid work as zero-cost operations.

### D. Open hardware means reproducible and replaceable modules

Start with hardware that has documented interfaces, regulatory/type-approval status and a mature open firmware option. Maintain artifact-level provenance: exact model and revision, BOM, board files when publishable, licensing status, firmware and binary blobs, build instructions, update/rollback, power measurements, enclosure/mount details, and replacement options. Mark each artifact open, partially open, closed, unknown or not applicable; “open hardware” is not one binary claim.

Treat a Starlink unit as an upstream IP/WAN adapter where the exact kit provides a suitable interface. Do not depend on undocumented private APIs or claim the terminal itself is open. Verify the exact kit, power, cables, bypass/bridge behaviour, plan and provider permission before a pilot.

### E. Intelligence advises; deterministic forwarding survives without it

Sol-Atlas can rank sites using measured demand, terrain, service gaps and backhaul availability. Symtropy can simulate placement, load, power, common-cause failures and restoration. Symthaea may advise caching, capacity procurement or energy allocation from bounded telemetry. The live packet-forwarding path must continue safely when every cognitive service is stopped or compromised. Mycelix governance and cooperative accounting must not become a single point of failure for emergency access. Luminous Platform owns reproducible configuration, evidence receipts, rollback and recovery.

## 3. Benchmark catalogue

All values below are **acceptance hypotheses**, not measurements or performance promises.

| ID / objective | Initial target | Minimum method and guardrails |
|---|---|---|
| Hard policy constraints | Zero violations | Enumerate cost cap, currency, stale observations, trust, authorization, data cap, task intent and quality-boundary negatives. |
| Decision explainability | 100% complete | Every candidate gets a deterministic selected/excluded result and machine-readable reasons; no-route must be explicit. |
| Determinism | 100% replay equality | Same versioned inputs and policy give the same decision structure regardless of candidate input order. Byte equality needs a later canonical serialization contract. |
| Data minimization | Zero content/history dependency | Static review and tests show policy decisions need declared task intent and link metadata, not packet/message contents or browsing history. |
| Cost per successful task | At least 30% lower than matched baseline | Compare fully burdened cost with same task quality/security/scope. Report numerator, denominator, failed tasks, currency and uncertainty. Treat as a stretch target to test, not a predicted outcome. |
| New-session WAN failover | 99% within 30 seconds over at least 100 trials | Inject physical link-down, upstream blackhole, DNS-only failure, gateway restart, primary recovery and rapid flapping. Report existing-session resets separately. |
| Local service continuity | 24 hours with all WANs disconnected | Keep local LAN and power available; test cable pull and upstream blackhole, then verify no hidden cloud dependency. Power-loss behavior is a separate test. |
| Service availability | 99.5% monthly target | Report local service, LAN, each WAN, and global internet independently; publish downtime windows and observation denominator. |
| Store-and-forward | At least 99% of unexpired test messages delivered after reconnection | Include partition, restart, duplicate, expiry, invalid signature, clock skew and recipient-unavailable cases. Report p50/p95 delay and duplicate count. Not a real-time emergency-telephony guarantee. |
| Whole-site energy | Baseline first, then minimize Wh per successful task | Meter terminal, router, AP, local node, conversion losses and backup power under idle, typical and peak load; report service results alongside watts. |
| Hardware traceability | 100% of critical artifacts with status recorded | Revision, license, firmware/blobs, BOM, build/recovery procedure and independent evidence link. Unknown is not equivalent to open or qualified. |
| Wi-Fi roaming | Baseline first | Measure median/p95 time from arrival to usable authenticated IP, failures, privacy, device/OS/federation and user cost. Requires real compatible AP/client and federation credentials. |
| Voice/video resilience | Baseline first | Separate call setup, new-call failover and mid-call handoff; record loss, jitter, interruption, codec and route cost. Define any MOS methodology before reporting it. |
| Cellular/eSIM | Blocked until exact provider/device/country entitlements are evidenced | No claim of arbitrary SIM selection, profile download, roaming access or coverage parity based on policy code alone. |
| Direct-to-phone satellite | Blocked until the exact service/device/country combination is verified | Distinguish emergency messaging, low-rate messaging and broadband. Check sky visibility and supported-country limitations. |
| Private LTE/5G | Blocked until spectrum, service and equipment authorization are documented | Open-source core/RAN code does not grant spectrum rights, device compatibility or permission to operate. |

For affordability, report more than one international basket: use ITU baskets for comparability; also report actual monthly cost and the fraction of household disposable income for multiple income bands, device cost/repair burden, and the amount of usable service delivered. An average can hide exclusion among lower-income households.

For economics, define fully burdened cost per task as:

> (upstream + amortized hardware + energy + site rent + maintenance + paid labour + support + tax + losses) / successful tasks

Show donations and unpaid labour in a parallel social-cost ledger rather than hiding them. Avoid treating “cost per GB” as sufficient: cache hits can save upstream bytes, but only useful successful outcomes tell us whether access improved.

## 4. Evidence ladder and immutable measurement protocol

1. **Static contract:** schemas, policy invariants, threat boundaries, firmware/license/BOM inventory.
2. **Pure Rust tests:** hard-gate cases, stable ordering, malformed metadata, duplicate IDs, explicit no-route, bounded-input behavior and mutation rejection.
3. **Reproducible simulation:** versioned topology and demand traces; seed, source SHA, configuration digest, assumptions, model limitations, and raw event log emitted together. Simulation is not field proof.
4. **Bench/VM tests:** isolated Ethernet topology with network namespace/VMs, controlled impairment and power measurement where possible. Label test doubles and virtual interfaces.
5. **Field trial:** named sites, dates, equipment/firmware, device mix, provider plans, consent/privacy procedure, weather/power/outage conditions and denominators.
6. **Independent review:** a second implementation or verifier validates receipts and negative controls; no self-declared qualification promotion.
7. **Deployment authorization:** separate sign-off for ISP resale/sharing, spectrum, type-approved radio equipment, physical installation, emergency claims and personal-data handling.

Every published score must be tied to exact repository/source revisions and immutable trace digests. Keep failed runs and negative controls. A queued or absent CI run is not a pass. No profile validator can prove the truth of measurements just because the values parse.

## 5. South African launch gates

Before connecting real customers, verify with ICASA and relevant counsel the precise classification of the intended network and service, applicable ECNS/ECS class or individual licensing or exemption process, resale/sharing terms from the upstream ISP, and equipment type-approval. ICASA's published guidance says exemptions may apply to small networks and identifies an exemption process for entities reselling upstream ECS; that guidance is not a determination for this particular deployment.

Reference:
- ICASA Licensing: https://www.icasa.org.za/pages/licensing
- ICASA Services Licensing and exemptions: https://www.icasa.org.za/pages/services-licencing

Also review power safety, mounting/fall risk, surge/lightning protection, batteries, electrical inspection, privacy, abuse reports, and emergency-service disclaimers. Keep the first pilot receive-only/local-first where permissions are uncertain and use already authorized upstream connectivity.

## 6. The first practical pilot

Build one small site with an OpenWrt-capable gateway, one documented access point, a local service/cache node, metered power, and one primary plus one genuinely diverse secondary upstream if lawful and affordable. A Starlink terminal may be one interchangeable WAN source, never the assumption on which the rest of the system depends.

First establish baseline values; then run the policy-core negatives and 100 failover trials; disconnect WAN for a 24-hour local service test; record energy and fully burdened cost; and only then invite a small, consented field cohort. Publish the exact topology, configs, bills, raw failure counts, device matrix and limitations. Expand only where measured cost per successful task and outage outcomes beat the matched baseline.

## 7. Explicit non-goals

This work does not claim free global internet, universal coverage, Google Fi parity, true multi-link bonding, live phone SIM/eSIM control, carrier agreements, Starlink authorization, spectrum permission, emergency telephony or any measured performance until the exact device/provider/site is tested.
