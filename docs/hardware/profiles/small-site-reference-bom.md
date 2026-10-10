# Small-Site Government / Regulated Office: Reference Bill of Functional Materials

**Profile:** one office/site, approximately 25–100 managed endpoints, unclassified administrative/business workloads, two WAN paths preferred, and local recovery capability.  
**Status:** planning baseline only; no exact purchasing BOM or deployment authorization  
**Research snapshot:** 2026-10-10  
**Architecture:** [Government Network Platform RFC 001](../architecture/government-network-platform-rfc-001.md)  
**Tracking:** [Issue #19](https://github.com/Luminous-Dynamics/luminous-platform/issues/19)

## 1. Assumptions and exclusions

This is a reference bill of *functional materials*, not a shopping list. It defines the components, quantities and evidence required before selecting exact models. Final quantities depend on floor plan, port count, Wi-Fi coverage, power, traffic, availability target, accessibility, budget, jurisdiction, data type and vendor-support constraints.

This profile is not designed for classified processing, tactical/mission networks, weapon interfaces, or combat operations. It makes no accreditation claim. The first build should be an isolated lab or non-production office pilot.

## 2. Hardware and physical bill

| ID | Component | Baseline quantity | Minimum requirements to capture | Candidate / disposition |
|---|---|---:|---|---|
| SITE-01 | WAN edge gateway/firewall | 2 for HA; 1 for initial lab | Supported firmware lifecycle; IPv4/IPv6; VLAN/VRF or equivalent zones; explicit stateful filtering; VPN/approved crypto only as required; config export; secure management; measured throughput at target traffic load; tested failover/recovery | No production model selected. OpenWrt One is a lab/reference candidate only; see the [evaluation plan](../hardware/plans/openwrt-one-evaluation-plan.md). Compare supported commercial and open-hardware options. |
| SITE-02 | Managed Ethernet access switch | 1–2 by redundancy and port count | Port count/PoE budget; managed L2/L3 features; VLAN/ACL/802.1X as required; useful telemetry; signed/controlled updates; configuration backup; secure management; current security support; compatible optics | No exact model selected. OCP networking is a design ecosystem, not one selected campus switch. |
| SITE-03 | Wireless access points | At least 2 for coverage testing; scale by survey | Applicable bands; enterprise identity (e.g. 802.1X/EAP); client isolation; guest/staff split; firmware updates; controller/API isolation; radio regulatory domain; roaming and coverage measurement | No model selected. Use a site survey rather than assuming a single AP covers the building. |
| SITE-04 | Compute/virtualization nodes | 1 for minimum pilot; 2 if local HA required | ECC memory where available; supported CPU/NIC/storage; hardware-backed identity where available; verified/measured boot; disk encryption; redundant PSU where needed; BMC isolated from production; firmware signed-update/recovery; reproducible OS install; thermal/power measurements | OCP designs are reference leads. Select exact current platform/SKU with supply, warranty and BMC evidence. |
| SITE-05 | Primary local storage | 1, sized from retention and workload | Capacity and IOPS from measurement; encryption/key plan; snapshots; disk/controller compatibility; failure alerts; supported firmware; restore procedure | No device selected. |
| SITE-06 | Independent backup target | 1 independent target plus offline/offsite copy according to policy | Separate administrative credentials/failure domain; immutable or write-protected copies where appropriate; encrypted backup; retention; verification and full restore tests; media-sanitization path | No device selected. Primary storage redundancy is not a backup. |
| SITE-07 | Managed user endpoints | One per assigned user plus agreed spares | Supported firmware/boot; full-disk encryption; endpoint identity; secure updates; compatibility/docking/accessibility; device management; repair/support and lifecycle; data sanitization | MNT Reform is an open-design reference, not a qualified office endpoint. Select exact models for supportability. |
| SITE-08 | User authenticators / smart cards | One per user plus a controlled replacement pool | Chosen identity standard; anti-phishing strength; issuance/revocation; reader/platform support; accessible recovery; inventory and loss process; required certification | No hardware selected. OpenSK is an implementation research lead, not a qualified token procurement choice. |
| SITE-09 | HSM / cryptographic module | Only if required by the selected policy or risk assessment | Exact module validation and configuration; key generation/import/backup/escrow; HA; custody/ceremony; access policy; lifecycle; supplier; recovery drill | No candidate selected. OpenTitan is IP; CrypTech is a historical research lead. Neither closes a required production HSM gap by association. |
| SITE-10 | Rack/cabinet and patch panels | 1 lockable rack/cabinet, sized after layout | Load rating; ventilation/airflow; grounding/bonding; lock/key policy; cable management; access clearance; local electrical/fire rules; labelled ports and asset location | OCP rack collateral is a design lead; exact rack and site fit not selected. |
| SITE-11 | UPS and power distribution | At least 1 UPS; redundancy as availability target requires | Measure peak and typical loads; required runtime; battery lifecycle; safe OS shutdown; management interface isolation; alarm behavior; electrical compatibility; service and replacement | No model selected. Size from measured loads; do not invent runtime from VA rating alone. |
| SITE-12 | Fibre/copper/optics | According to port map, distance and uplink speeds, plus spares | Cable category/fibre type; link distance; connector; transceiver compatibility; DOM/telemetry; certification test; routing/labeling; spare modules and patch cords | No site BOM; exact standards and part numbers follow site survey and chosen switches. |
| SITE-13 | Out-of-band console/management | 1 isolated method for recovery; redundant where required | No Internet exposure; separate management path; MFA/role control where supported; auditable use; locked physical access; firmware updates; serial/KVM compatibility; break-glass procedure | No candidate selected. Never make recovery depend solely on the normal production network. |
| SITE-14 | Environmental/power sensors | 1+ per rack/room as layout requires | Temperature/humidity; leak/fire/alarm needs determined by site; calibration; alert path independent of monitored component; secure management; battery replacement | Open sensor PCBs may be appropriate for non-safety-critical monitoring after qualification. Life-safety systems remain code-compliant certified equipment. |
| SITE-15 | Backup WAN | 1 independent circuit or approved alternate medium | Provider/path diversity, contract, coverage, policy, data limits, antenna/radio rules, security controls, failover/revert tests | Optional, jurisdiction-dependent. No cellular/satellite/radio SKU selected. |
| SITE-16 | Critical spares and recovery kit | At least one of each agreed critical single point of failure | Spare PSU, supported storage drive, compatible NIC/optics/cables, a spare AP/gateway where justified, labelled console cables, offline recovery media, encrypted configuration backup and contact/escalation list | Part numbers must match the selected hardware BOM. |
| SITE-17 | Secure retirement/sanitization | One documented process/tool set for each media class | Media inventory; approved sanitization method; verification; key destruction; custody evidence; approved recycling/disposal | No tool/process selected; policy must be chosen before first data-bearing system is retired. |

## 3. Software and service dependencies (not hardware SKUs)

- NixOS base images and a pinned, signed or integrity-checked artifact cache.
- Device/asset inventory, owner, hardware revision, firmware release, patch status, supplier and support end date.
- Identity provider integration, MFA, device and service identities, certificate issuance/rotation/revocation, privileged-access process and tested emergency access.
- DNS/DHCP/IPAM/time services and their recovery plan.
- Network configuration contracts, default-deny rules, machine-readable allowed flows and automated negative-path checks.
- Nixward/sovereign-boot/luminous-edge integration under explicit authority and recovery constraints.
- Logging/monitoring and alert transport with bounded retention and data minimization.
- Backup catalog, immutable/offline copy, restore drills and recovery time/objective records.
- Incident response, vulnerability intake, patch windows, software/firmware rollback, and media sanitization.
- Optional Xenia communications integration only after selecting and qualifying user devices, codecs, updates, privacy and interoperability.

## 4. Minimum design artifacts before ordering

- Site drawing, rack elevation and hardware placement.
- Logical network diagram, address plan, VLAN/VRF table and explicit permitted-flow matrix.
- Port schedule, cable labels, fibre distance/speed, PoE budget and spare optics.
- Workload and traffic measurement plan; throughput, latency and availability targets.
- Power budget by device, startup and peak power, UPS runtime calculation, cooling/airflow, grounding and local-code review.
- Exact BOM with manufacturer part number, revision, license/source links, source country/provenance where evidenced, supplier quote/date, lead time, warranty, support end, alternates and spares.
- Firmware/boot/support matrix; secure update and recovery method; management-plane threat model.
- Identity/key management design; no secret keys embedded in configuration or published evidence.
- Test plan for unauthorized flows, failed identity, WAN/switch/gateway loss, power interruption, certificate expiration, offline operation, restore and device replacement.
- Customer-specific control mapping, required independent tests and explicit authorization owner.

## 5. Pilot acceptance gate

The pilot is ready for limited non-production use only after:
1. Exact hardware models/revisions and firmware are recorded, with current supply/support evidence.
2. All intended and forbidden network paths have executed test evidence.
3. Gateway, switch, identity and power failure have expected outcomes; recovery instructions have been rehearsed.
4. Updates are verified and recovery/reinstall paths work on the exact revisions.
5. Backup restore, account revocation, certificate rotation and hardware replacement are demonstrated.
6. Security and privacy reviewers have accepted the profile's scope and limitations.
7. The deployment report identifies every untested requirement; queued or skipped tests do not count.

The profile remains **not procurement ready** until exact SKUs, quantities, supplier records and customer requirements are pinned.
