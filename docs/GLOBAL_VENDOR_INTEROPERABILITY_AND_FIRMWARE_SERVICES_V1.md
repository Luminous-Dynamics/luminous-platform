# Global Vendor Interoperability and Firmware Services v1

**Reviewed:** 2026-10-10  
**Status:** architecture and service policy; vendor/model support is not yet qualified  
**Scope:** globally deployable, jurisdiction-profiled network and security infrastructure management  
**Related:** [Government & Regulated Infrastructure product line](GOVERNMENT_NETWORK_PRODUCT_LINE_V1.md), [hardware portfolio](../hardware/portfolio-v1.json), [South African procurement and single-site execution track](SA_PROCUREMENT_AND_SINGLE_SITE_EXECUTION_TRACK_V1.md), [control evidence matrix](GOVERNMENT_NETWORK_CONTROL_EVIDENCE_MATRIX_V1.md)

## 1. Decision

Build a **vendor-neutral fleet security and lifecycle platform** with narrow, explicitly qualified adapters. Do not try to build one universal firmware image or pretend that a brand name alone determines compatibility.

The service should manage mixed estates containing Cisco IOS/IOS XE/NX-OS, Cisco Meraki cloud-managed appliances, Palo Alto PAN-OS/Panorama, NETGEAR product families, OpenWrt, SONiC and other vendors over time. These are separate platform families: Cisco enterprise IOS devices are not Meraki devices; NETGEAR consumer, smart-managed, fully managed, and Insight-managed families do not share one management contract; firmware and APIs vary by SKU, hardware revision, software train and entitlement.

**Default approach: manage first; reflash only as a specifically qualified exception.** We can offer useful global services without replacing an OEM operating system: asset inventory, version and support-lifecycle tracking, configuration backup, policy validation, authorized configuration changes, vendor-native signed upgrades, recovery planning, security evidence and fleet reporting. Alternative firmware/OS installation is a separate opt-in service limited to exact device/revision combinations with published support and a tested restore method.

## 2. Global architecture: portable core, local adapters, jurisdiction overlays

### Portable core

The same core capabilities should be useful across jurisdictions:

- Asset identity and exact-model discovery; manufacturer, SKU, serial where appropriate, hardware revision, software train, bootloader, support/entitlement, end-of-sale/end-of-support and dependency records.
- Desired-state policy as versioned, typed intent—not shell snippets executed with ambient root or a generic administrator token.
- Read-only discovery by default, then a plan showing the exact before-state, intended diff, policy evaluation, blast radius, expected outage and tested recovery.
- Explicit approval bound to the exact target identities, action digest, configuration pre-state, maintenance window and expiry. A stale pre-state or changed diff invalidates the approval.
- Vendor-specific application and verification; explicit timeouts, idempotency keys, uncertain-outcome handling and a durable effect journal. A timed-out API call is not automatically safe to replay.
- Configuration and firmware provenance, evidence receipt generation, offline restore, signed audit artifacts and lifecycle alerts.
- A clear support matrix: model + revision + software version + region + method + tested operations + known limitations. Unknown combinations remain unsupported or discovery-only.

### Adapter boundary

Every adapter is untrusted until qualified. It must implement only capabilities tested for the exact platform. A common interface may expose operations such as `discover`, `read_state`, `export_config`, `plan_diff`, `validate_candidate`, `apply_config`, `verify_state`, `restore_config`, `check_firmware_entitlement`, `stage_vendor_firmware`, `verify_boot_health` and `collect_evidence`. Unsupported operations must return an explicit `unsupported` result—not silently fall back to a different method.

Each operation should be characterized as one of:
- **Read-only:** inventory, facts, version, health and evidence collection.
- **Reversible configuration:** apply a validated diff with a proven rollback path.
- **Disruptive lifecycle:** reboot, firmware install, restore, factory reset or image change.
- **High-consequence change:** routing, ACL, authentication, management-plane or security-policy modification.
- **Reflash/conversion:** replacement of the platform's normal firmware/OS; disabled unless the exact device is approved for it.

No general-purpose adapter should have unrestricted command execution as its normal interface. Where a vendor provides only an interactive CLI, use command templates, a bounded parser, a tested state-machine and exact allowlists. Never accept model output or natural language as executable authority.

### Jurisdiction overlay

Global means *portable across jurisdictions*, not identical legal claims worldwide. Keep one technology/control vocabulary and attach separately versioned jurisdiction/customer overlays. Overlays may specify privacy/data residency, crypto/import/export restrictions, radio/equipment approval, product security/product liability, accessibility/language, records retention, critical-infrastructure controls, procurement eligibility, support location, local representative/importer and acceptable cryptographic validations.

NIST CSF 2.0 can serve as a vendor-neutral risk/outcome crosswalk for many organization types; it is guidance, not a universal legal certification or substitute for local obligations. The EU Cyber Resilience Act has separate application dates: vulnerability/incident reporting obligations apply from 11 September 2026, while most provisions apply from 11 December 2027. The applicability to a specific product/service and the legal role of the supplier must be assessed; don't infer it from the existence of this platform alone. Sources:
- NIST CSF 2.0: https://www.nist.gov/cyberframework
- EU Cyber Resilience Act, Regulation (EU) 2024/2847: https://eur-lex.europa.eu/legal-content/EN/TXT/?uri=CELEX%3A32024R2847
- EU Commission CRA summary: https://digital-strategy.ec.europa.eu/en/policies/cra-summary

## 3. Vendor support policy (initial research baseline)

These are *integration priorities*, not a claim that any adapter is implemented or tested. Firmware and licensing facts should be rechecked against the customer's exact model, software train, support contract and region before each job.

| Platform family | First-class service path to build | Update / licensing boundary | Alternative OS / reflash policy |
|---|---|---|---|
| **Cisco IOS / IOS XE / NX-OS** | Read-only inventory; API/NETCONF/RESTCONF/SSH where supported; sanitized config backup; policy and drift review; staged config; image/version/support-lifecycle checks; customer-authorized upgrade and rollback plan | Obtain software images and entitlement through authorized channels. Respect Smart Licensing, export controls, feature entitlements, support contracts and train-specific lifecycle dates. Cisco publishes per-release lifecycle and EOL statements; never assume a familiar version remains security-supported. | **Vendor-native only by default.** Do not assume a Cisco-branded SKU is supported by SONiC/OpenWrt. Alternative NOS only for an exact hardware platform explicitly listed by the NOS project and supported by a lawful source/supply path, with customer approval and a tested boot/recovery method. |
| **Cisco Meraki (MX/MS/MR and related families)** | Separate Meraki Dashboard API adapter, organization/network scoped credentials, region-aware API endpoints, inventory, config and health, licensing visibility, staged maintenance windows and audit evidence | Meraki documents centrally managed firmware upgrades through Dashboard, and its license documentation says each hardware component needs a cloud license to be managed. Confirm license tier, subscription status, API terms, cloud/region dependency and actual behavior during WAN/cloud outages. | **No generic flashing service.** Use the supported Dashboard lifecycle. Do not attempt to convert Meraki units to another OS, bypass cloud/license behavior or defeat boot restrictions unless a specific documented and lawful supported route exists and the customer explicitly approves the support consequences. Current default is `not_supported`. |
| **Palo Alto Networks PAN-OS / Panorama** | API-backed inventory and policy export; candidate diff review; staged config/commit; post-commit health and policy verification; support/license and content-update evidence | Use official PAN-OS/Panorama APIs and the entitled vendor firmware/content-update process. Palo Alto documentation says PAN-OS software updates require a valid Support license. Treat licensing, subscriptions, image integrity, threat/content updates and rollback as separate checks. | **Vendor-native only by default.** Do not replace PAN-OS with OpenWrt/SONiC or modify firewall firmware to bypass subscriptions, secure boot or support constraints. Lab/recovery work must use the exact supported image, prerequisites and approved maintenance process. |
| **NETGEAR** | Separate adapter profiles by family: consumer/home, smart-managed, fully managed/enterprise, AV-line and Insight-managed. Begin with read-only discovery, config export, version/lifecycle checks and documented update workflows. | Warranty/support vary by country, SKU and purchase channel. NETGEAR says business warranty eligibility depends on authorized-reseller purchase and original-purchase documentation. Verify exact regional terms, serial/SKU, cloud-management requirements and support route. | **Selective and model-specific.** OpenWrt support exists for some NETGEAR models; it is not a brand-wide compatibility statement. Require exact model/hardware revision, current OpenWrt device page, documented install procedure, firmware image identity, known recovery method and a tested restore. Do not assume third-party firmware preserves OEM support or warranty. |
| **OpenWrt-supported devices** | Native OpenWrt configuration/lifecycle adapter, reproducible config backup, package/version inventory, signed release verification, staged sysupgrade and tested recovery | Use the exact device page and supported release; assess proprietary Wi-Fi blobs, bootloader behavior, package feeds and device-specific caveats. Vendor stock availability and upstream firmware support are different things. | **Approved target for selective conversion**, only where exact hardware revision and install/restore instructions are confirmed. Record stock firmware backup where lawful and possible; test recovery before production use. |
| **SONiC-supported switches** | Separate datacenter/network operating-system adapter, capability profile per ASIC/platform/NOS image, tested port/optic/PHY/config and telemetry mapping | The SONiC project documents platform-specific support and describes SONiC as software rather than a hardware product. Community support is not automatically a vendor SLA. Identify a supplier/OEM that explicitly supports the chosen chassis, ASIC, image, optics and lifecycle. | **Datacenter-class, platform-specific option.** Use only devices on the maintained supported-platform list with a qualified supplier, validated image/build and recovery path. Do not install SONiC on a device merely because it is a switch. |
| **Other manufacturers** | Add adapters after the common contract and evidence model work; prioritize standards-based management where available, then officially documented vendor APIs/CLIs | Preserve each manufacturer's entitlement, support, API terms, regional cloud endpoint, model support and lifecycle semantics. Avoid assuming two products with the same protocol have identical behavior. | Model-level allowlist only. No blanket flash eligibility for a manufacturer or product family. |

Official vendor/source references:
- Cisco IOS XE Smart Licensing Using Policy: https://developer.cisco.com/docs/ios-xe/smart-licensing/introduction/
- Cisco IOS XE software lifecycle: https://www.cisco.com/c/en/us/products/collateral/ios-nx-os-software/ios-xe-26/bulletin-c25-2378701.html
- Cisco software lifecycle and supported trains: https://www.cisco.com/c/en/us/support/lifecycle/software.html
- Meraki firmware upgrade process: https://documentation.meraki.com/Platform_Management/Product_Information/Compatibility_and_Firmware/Firmware_Upgrades/Cisco_Meraki_Firmware_FAQ
- Meraki licensing FAQ: https://documentation.meraki.com/Platform_Management/Product_Information/Meraki_Licensing/General_Licensing_Information/Meraki_Licensing_FAQs
- Meraki Dashboard API: https://developer.cisco.com/meraki/api-v1/
- PAN-OS API overview: https://docs.paloaltonetworks.com/ngfw/api/get-started-with-the-pan-os-rest-api
- PAN-OS Support-license/update requirement: https://docs.paloaltonetworks.com/pan-os/11-0/pan-os-admin/subscriptions/activate-subscription-licenses
- NETGEAR warranty/support: https://kb.netgear.com/000055960/NETGEAR-warranty-and-support-offerings and https://kb.netgear.com/app/answers/detail/a_id/12171
- OpenWrt install prerequisites and exact-device guidance: https://openwrt.org/docs/guide-quick-start/factory_installation and https://openwrt.org/toh/start
- NETGEAR models indexed by OpenWrt: https://openwrt.org/toh/hwdata/netgear/start
- SONiC platform support model: https://github.com/sonic-net/SONiC/wiki/FAQ and https://github.com/sonic-net/SONiC/blob/master/Supported-Devices-and-Platforms.html

## 4. Should Mycelix / Holochain be part of security?

**Yes—as a distributed trust, provenance and evidence layer. No—as the only identity provider, packet-policy enforcement point, firmware verifier, or real-time authorization authority.**

Holochain provides agent-centric source chains and a peer-validated DHT. Its integrity zome validation rules are expected to be deterministic; peers validate shared public records. Holochain documentation also makes clear that it does not attempt to maintain consensus over a single global state. This is useful for auditable collaboration and distributed evidence, but it does not magically prove the factual truth of a device report, protect a compromised endpoint, or enforce a firewall ACL. Source: https://developer.holochain.org/build/validation/, https://developer.holochain.org/concepts/7_validation/, https://developer.holochain.org/concepts/1_the_basics/

### Appropriate Mycelix/Holochain uses

- **Asset and supplier provenance:** signed assertions about manufacturer, SKU, revision, serial-token commitment, ownership/custody transfers, sourcing and lifecycle evidence.
- **Firmware and configuration receipts:** signed digest of the exact artifact/config, source and image provenance, verifier identity, decision, measured result and timestamp/nonce context.
- **Change authorization and witness trail:** who requested, approved, executed and independently evaluated a change; policy/profile revision; action digest; before-state and resulting-state digests; evidence references.
- **Cross-organization trust:** supplier attestations, service-partner responsibilities, maintenance custody, incident evidence exchange, independent evaluation and dispute/supersession history.
- **Evidence replication:** help make tampering, equivocation and unsupported claim histories detectable, while keeping customer evidence under the appropriate privacy and retention policy.

### What must remain outside Holochain's trust boundary

- Device, API, SSH, SNMP, cloud and HSM credentials; private keys; recovery codes; secrets; live full configurations; unredacted network maps; personal/customer data. Holochain documents that public records are shared to the DHT and that source-chain actions are public even where an entry's content is private. Keep sensitive evidence in customer-controlled encrypted stores and publish only carefully reviewed, minimized commitments/references. A hash or serial commitment can still leak information if it is guessable or correlatable.
- The actual packet path, switch ACLs, local authorization checks, secure boot chain, firmware signature verification, TPM-backed keys, FIDO2 authentication and OS process isolation. Those require local or vendor-native enforcement.
- A global always-online dependency for safe site operation. A site must continue safe local operation during WAN, Holochain, identity, or regional API outages; new changes can queue, but unsafe changes must not execute just because replication is unavailable.
- A claim that “multiple peers validated this entry” proves the hardware was genuine or the measured event really happened. Evidence needs a trustworthy sensor/test harness, cryptographic signatures, freshness challenges, exact subject binding and independent assessment. Holochain validates that records follow app rules; it cannot independently observe the physical device.

### Recommended architecture

1. **Local enforcement agent:** evaluates signed, versioned policy bundles; verifies target identity, authorization, action digest and fresh pre-state; enforces deny-by-default change permissions; performs bounded API/CLI operations; records a durable effect journal; and recovers safely from indeterminate outcomes.
2. **Device/vendor trust:** signed vendor firmware or approved alternative image; secure/measured boot where available; hardware-backed keys/TPM where present; authenticated vendor APIs; minimum required privileges; firmware and support lifecycle checks.
3. **Evidence verifier:** independently verifies signatures, artifact digests, timestamps/freshness, logs and test results; does not simply trust a change executor to grade its own work.
4. **Mycelix/Holochain trust fabric:** records typed, signed, privacy-minimized claims and receipts; applies deterministic integrity validation; coordinates witnesses and cross-organization provenance; retains supersession and dispute history.
5. **Customer authority:** final policy ownership and deployment authorization remain with the customer or legally responsible authority. A Holochain record can carry that authority's signed decision but must not manufacture it.

The system should bind any shared evidence record to the exact device/model/revision, firmware, profile digest, configuration/action digest, source commit, test version, signer identities and raw-artifact digests. Missing, stale, unverifiable, or conflicting records must not promote a device or change to qualified. If Holochain is unreachable, existing approved local policy remains enforceable; no new global trust claim is silently assumed.

## 5. Should we offer a firmware flashing service?

**Yes, but launch it as a controlled firmware lifecycle and recovery service first, then offer alternative-OS conversion only for an explicit allowlist.** "Flash any hardware" would be an unsafe commercial promise. Some devices require vendor entitlements, rely on cloud controllers, have signed-image checks, use model-specific boot chains, or lose OEM supportability under unauthorized modifications. The exact terms vary; assess the exact platform and written customer consent.

### Service catalogue

**F1 — Firmware posture assessment (first release).** Inventory running versions; map advisories and vendor lifecycle status; check support/license status; provide a signed report, recommended target version and maintenance plan. Read-only access by default.

**F2 — OEM firmware maintenance.** Verify authorization and entitlement; obtain the correct image through the vendor/customer-authorized channel; verify provenance/signatures/hashes; back up config and license state where permitted; validate prerequisites and free storage; stage canary/maintenance window; upgrade; verify boot, interfaces, routing, policies, HA and monitoring; collect before/after evidence. Stop if image provenance or entitlement is uncertain.

**F3 — Recovery and reimage.** Recover supported bricked/failed units using OEM-documented recovery methods, authorized service tools and customer-approved credentials. Preserve chain of custody; document data/config loss; never bypass a cloud subscription or cryptographic access control. A hardware replacement/RMA may be the only safe path.

**F4 — Alternative OS conversion (limited beta).** Convert only listed model + hardware revision + flash/bootloader + region combinations to an explicitly supported OpenWrt or SONiC target. Requires a signed job authorization, independent preflight review, verified image, power-stable staging, complete recovery method, a known-good vendor image where lawful, explicit support/warranty disclosure and customer acceptance. Perform initial work in a lab or on customer-owned non-production hardware; no first-time conversions on production firewalls or the only site gateway.

**F5 — Fleet firmware governance.** Risk-ranked fleet plan, staged rings, canary, change windows, health-gated rollout, automatic stop conditions, exception tracking, and evidence packets. A reported API success is not enough: verify post-upgrade behavior on the device.

### Mandatory flashing preflight

Do not start unless all applicable items have evidence:

- Written owner/customer authorization identifying the exact physical asset and allowed operation.
- Exact manufacturer/model/SKU, PCB/hardware revision, bootloader, installed version and region; no guesses from chassis appearance alone.
- Vendor support/entitlement and warranty impact checked; customer understands where OEM TAC/RMA support may be affected.
- Correct image, trustworthy origin, immutable version and digest; signature verified if the platform provides a verification path; licensing terms checked.
- Configuration backup, certificates/keys/license state handling and sensitive-data custody documented, without exporting secrets unnecessarily.
- Tested recovery path and required serial/JTAG/recovery hardware available where lawful; alternate console/replacement device ready.
- Measured power stability, maintenance window, impact plan and rollback decision threshold.
- Exact capability/test matrix shows the target is supported; no downgrade, export-control or anti-circumvention issue is being bypassed.
- Before/after logs, firmware and config hashes, final health checks, exceptions and customer sign-off are captured.

If a vendor's locked boot chain, cloud license, entitlement or API terms prevent the requested operation, decline that method and offer the supported vendor-native maintenance/recovery route. Do not promise universal conversion for Meraki, Cisco, Palo Alto or NETGEAR as brands.

## 6. Suggested implementation roadmap

**Phase A — Common control contract (now).** Define versioned adapter capabilities, typed results, authorization receipt, exact-state preconditions, change-diff preview, idempotency/effect journal semantics and evidence packet schema. Maintain read-only-only adapters until that contract is tested.

**Phase B — Vendor-native MVP.** Implement one limited path per family: (1) Cisco IOS XE read-only inventory + config export; (2) Meraki Dashboard inventory/licensing/status via regional API base; (3) PAN-OS read-only inventory + configuration export using official APIs; (4) NETGEAR family discovery and version/config backup where the exact model supports it. No automatic firmware rollouts in this phase. Use lab/sandbox accounts and explicitly supported models.

**Phase C — Authorized low-risk changes.** Add candidate diff, authorization binding, idempotency, rollback and evidence receipts. Start with non-disruptive changes in simulation and dedicated lab devices, then tightly controlled canaries. Configuration changes should not be inferred to be safe merely because an API accepts them.

**Phase D — OEM firmware operations.** Add device-family-specific prerequisites, image provenance, licensing checks, backup/recovery and post-upgrade state verification. Track lifecycle and entitlements as product data.

**Phase E — Alternative OS allowlist.** Start with a small number of OpenWrt models that can be physically sourced and restored; add SONiC only for exact supported platforms with a credible OEM/ODM support arrangement. Expand by model and firmware revision only after tests, not by logo.

**Phase F — Independent trust integration.** Use Mycelix/Holochain to record verified signed evidence and cross-party receipts. First establish local security contracts and the independent verifier. Distributed evidence is an improvement to auditability and trust coordination, not a replacement for local enforcement.

## 7. Release maturity and product promises

Use an additional, per-adapter maturity track distinct from hardware H0–H5:

- **A0 — Discovered:** vendor/model identified, API and source documentation linked.
- **A1 — Read-only tested:** exact model/firmware inventory and config export observed in a lab.
- **A2 — State contract tested:** schema, authorization, diff, precondition and failure/retry behavior tested.
- **A3 — Change qualified:** low-risk changes and recovery work on exact devices; negative cases and timeouts recorded.
- **A4 — Firmware/recovery qualified:** exact image, upgrade and recovery path passed on physical units.
- **A5 — Serviceable:** documented global/regional support, license/entitlement handling, field procedures, customer contract and escalation path exist.

This is not a product-security certification. A vendor adapter can be A1 while the hardware itself is not H4; a device can be vendor-supported while an adapter is not qualified. Publish those statuses separately.

**Initial recommendation:** build a secure global vendor-native management and evidence platform; use Mycelix/Holochain for provenance and distributed evidence with privacy safeguards; offer OEM firmware maintenance/recovery as a controlled service; and make alternate-OS flashing a separate, explicit, model-by-model capability.
