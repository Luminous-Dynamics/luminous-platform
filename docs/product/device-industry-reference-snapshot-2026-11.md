# Luminous Device Family — Industry Reference Snapshot (2026-11)

**Status:** Procurement and benchmark research; illustrative comparators, not a vendor ranking  
**Date checked:** 2026-11-09  
**Parent:** [Luminous device family RFC #349](https://github.com/Luminous-Dynamics/luminous-platform/issues/349)  
**Architecture:** [device-family architecture 0.1](./luminous-device-family-architecture-0.1.md)  
**Benchmark method:** [device workload programme 0.1](../../benchmarks/luminous-device-workload-v0.1/README.md)

## Why this snapshot exists

We need to compete against actual products and mature ecosystems, not caricatures of incumbents. These examples establish capability floors and comparison candidates. They do not prove like-for-like performance, and the manufacturer figures below are not Luminous measurements.

## 1. Mobile: repairability and support already have serious competitors

**Fairphone (Gen. 6 and Gen. 6+)** are strong longevity and repairability references. Fairphone's current Gen. 6+ profile uses Snapdragon 7s Gen 4 and 12 GB RAM; the Gen. 6 uses Snapdragon 7s Gen 3 and 8 GB RAM. Both have 12 user-replaceable parts, including battery, display, USB-C port and back cover; Fairphone states that several common modules can be replaced in under five minutes and parts/software support extend to 2033. The manufacturer lists a five-year warranty. Its security description lists a Trusted Execution Environment and says there is no hardware secure element beyond the eSIM chip. That distinction matters when mapping actual trust primitives rather than treating “secure” as a generic badge.

Sources:
- https://www.fairphone.com/the-fairphone-gen-6
- https://us.fairphone.com/the-fairphone-gen-6-plus
- https://www.fairphone.info/the-fairphone-gen-6-plus
- https://support.fairphone.com/hc/en-us/articles/9979180437393-How-long-will-my-Fairphone-receive-software-and-security-updates

**Implication for Luminous:** A repairable phone and long software support are not novel differentiators by themselves. The opportunity is to combine those qualities with user-controlled local intelligence, offline operation, transparent sensor/data policy, independently appraised boot/device evidence, cross-device workload delegation and cryptographically linked repair/requalification records. We should compare repair cost and time against Fairphone directly on exact parts and procedures.

Do not promise an open mobile boot chain, custom modem or custom phone board until the exact OEM platform, drivers, radio certification and supplier lifecycle are known.

## 2. Laptops and workstations: repairability is entering mainstream business hardware

**Framework Laptop 13 Pro (2026)** is a high-value repairability, upgradeability and Linux-support comparator. Framework lists Core Ultra Series 3 options, up to 64 GB LPCAMM2, NVMe storage up to 8 TB, a 74 Wh battery, repairable/upgradeable design and Ubuntu preloaded options. Its advertised 20-hour battery result is a vendor test under a defined Netflix 4K / 250-nit / 30%-volume / Windows 11 profile; it must not be compared directly with a Luminous mixed-use or local-AI runtime unless the test is matched.

Sources:
- https://frame.work/laptop13pro
- https://frame.work/blog/introducing-framework-laptop-13-pro

**Lenovo ThinkPad P16s Gen 5** is a current professional workstation comparator. iFixit's 2026 assessment reports upgradeable LPCAMM2 memory up to 96 GB, one M.2 SSD slot, customer-replaceable 60 or 90 Wh battery, and straightforward display, keyboard and fan replacements; some ports remain soldered. Lenovo's repair guide lists 36 service parts for the AMD variant, including a replaceable USB-C module. Exact Intel/AMD machine type matters: 21XE/21XF and 21XG/21XH are distinct SKUs.

Sources:
- https://www.ifixit.com/repairability/laptop-repairability-scores
- https://www.ifixit.com/Device/Lenovo_ThinkPad_P16s_Gen_5
- https://pcsupport.lenovo.com/us/en/products/laptops-and-netbooks/thinkpad-p-series-laptops/thinkpad-p16s-gen-5-type-21xg-21xh/21xh/selfrepair/removalsreplacements

**Implication for Luminous:** Competing on “modular laptop” alone is insufficient. We need a stronger combined result: repairability, sustained local Symthaea workload performance, whole-device joules per accepted task, good Linux/firmware lifecycle, offline recovery, privacy-preserving workload placement and a validated thermal model. Use Symthaea LAPTOP-ARCH-001 #8446 and Symtropy LAPTOP-TWIN-001 #2697 as the canonical comparison owners.

## 3. Field devices: environmental and service capabilities are mature

**Panasonic TOUGHBOOK G2 (FZ-G2)** is a reference for field tablet engineering and enterprise support. Panasonic's current product page lists an Intel Core Ultra 5 135U with NPU, 10.1-inch 1,000-nit glove-compatible display, optional expansion modules, IP65, MIL-STD-810H and MIL-STD-461G test claims, and Red Hat Enterprise Linux options. Panasonic lists up to 15 hours under MobileMark 2025 and a three-year limited warranty. These figures are model- and configuration-specific vendor claims; obtain the underlying test reports and match the workload before comparison.

Sources:
- https://connect.na.panasonic.com/toughbook/rugged-laptops-tablets/toughbook-g2
- https://connect.na.panasonic.com/toughbook/how-to-choose-rugged-laptops-tablets

**Implication for Luminous:** Do not enter field hardware with a rugged-looking case and a marketing IP claim. Start by qualifying an established rugged OEM device. Our differentiation should be offline-first workflow and cognition, authenticated networking, exact hardware/configuration identity, repair-triggered requalification and lifecycle evidence. A MIL-STD-810H reference is a tailored environmental engineering process, not a universal pass stamp; choose the methods to match declared use and publish the actual test results.

## 4. Servers and edge nodes: raw performance is a very high bar

**AMD EPYC 9965** is the top-end 9005-series reference with 192 cores / 384 threads, 12 DDR5 memory channels, up to 614 GB/s per-socket memory bandwidth and a 500 W default TDP. This is an important reminder that raw core count is not equivalent to efficiency or affordable edge compute; compare exact model and full-system power.

**Dell PowerEdge R7725** is a concrete 2U server reference supporting two EPYC 9005 processors on supported configurations. It is a platform-management and serviceability comparator, not a single CPU benchmark point. Record the exact chassis, CPU, DIMM, storage, NIC and firmware configuration.

Sources:
- https://www.amd.com/en/products/processors/server/epyc/9005-series/amd-epyc-9965.html
- https://www.amd.com/en/partner/articles/epyc-9005-series-processors.html
- https://www.dell.com/support/product-details/en-us/product/poweredge-r7725/

**Open Compute Project Server** is a useful mechanical/electrical/process reference across open rack-compatible chassis, sleds, blades, microservers, management/debug/test interfaces, manufacturing and decommissioning.

Source: https://www.opencompute.org/projects/server/

**Implication for Luminous:** We should not set “more CPU cores” as the goal. Buy/qualify proven server platforms and measure Symthaea throughput, accepted-task joules, idle/loaded power, tail latency, failure recovery, operational serviceability and cost per verified operating hour. The edge Node product should compete on evidence-qualified autonomy for distributed sites, not on recreating Dell's server portfolio.

## 5. XR: reuse the runtime and optics ecosystem

**HTC VIVE Focus Vision** is a useful enterprise XR reference. The official specification lists dual 2448 × 2448 LCD panels, up to 120° FOV, 12 GB LPDDR5 RAM, 128 GB ROM and microSD expansion up to 2 TB. Its 120 Hz DisplayPort mode is labelled Beta; do not treat it as a universally available standalone mode. Verify the exact commercial SKU, firmware, tracking-sensor set, battery configuration and runtime before comparing it with another headset.

Sources:
- https://business.vive.com/eu/product/vive-focus-vision/
- https://www.vive.com/sea/product/vive-focus-vision/specs/

The OpenXR Conformance Test Suite is necessary evidence for runtime API conformance, but it is not evidence of tracking accuracy, end-to-end motion-to-photon latency, spatial mapping accuracy, comfort, optics or safety.

Sources:
- https://github.com/KhronosGroup/OpenXR-CTS
- https://registry.khronos.org/OpenXR/conformance/cts_usage.html

**Implication for Luminous:** Start with a supported OpenXR device and make Sol-Atlas spatial understanding, privacy, calibration provenance and Symtropy-backed physical-model alignment the differentiators. Build custom optics or headset silicon only after a measured gap. Treat tracking/pose accuracy and motion-to-photon as distinct physical metrics.

## 6. Shared tables, desktops and household nodes

For Luminous Table, Desk, Hearth and Commons, the most economical first strategy is composition: commodity displays/touch surfaces plus qualified modular compute, standard interfaces and Luminous platform software. There is no reason to design a bespoke panel, touch controller or all-in-one enclosure before the multi-user workflow, service model and physical requirements have been tested.

The same logic applies to Hearth and Gateway: qualify standard networking/storage components, then differentiate through local-first integration, device identity, owner-controlled policy and reliable offline operation.

## 7. Where the competitive opening actually is

| Dimension | Strong current industry references | Luminous should aim to add |
|---|---|---|
| Repairable mobile | Fairphone | Repair + device identity + post-repair qualification + cross-device local intelligence |
| Modular laptop | Framework; selected ThinkPad configurations | Workload-first capability/energy/thermal qualification and evidence-backed lifecycle support |
| Rugged field computing | Panasonic TOUGHBOOK and other rugged OEMs | Offline-first workflows, authenticated site mesh, sensor/authority separation and exact repair receipts |
| Servers | Dell, HPE, Lenovo, Supermicro, AMD/Intel platforms; OCP specifications | Verifiable local autonomy, multi-vendor workload placement, evidence-qualified recovery and operational TCO |
| XR | Meta, HTC VIVE, Varjo and OpenXR ecosystem | Sol-Atlas ground-truth spatial evidence, local privacy and physical-model reconciliation |
| AI acceleration | NVIDIA, AMD, Intel, Qualcomm and mobile SoC vendors | Actual backend execution and useful correct work per joule on Symthaea's own workloads |
| Device lifecycle / security | OEM firmware, TPM/DICE, enterprise management and update systems | One transparent product-level evidence graph without creating a replacement firmware or security root |

## 8. What would count as genuinely better

Luminous should be able to demonstrate all of the following on at least one exact supported profile before claiming differentiated superiority:

1. **Matched useful-work efficiency:** lower joules per correct accepted Symthaea task at the same or better quality, latency target and thermal envelope than the best tested comparable device.
2. **Predictable sustained behavior:** physical traces validate Symtropy's power/thermal and throughput prediction on a held-out test set, with calibrated uncertainty and explicit abstention outside its domain.
3. **Repair with evidence:** a repair changes only the claims it can invalidate; those claims remain suspended until the exact relevant retests pass; receipts are independently verifiable offline.
4. **Sovereign local operation:** core tasks remain usable without a vendor cloud account or internet; data is not offloaded to an unauthorized peer, and a network partition cannot manufacture success or authority.
5. **Lifecycle advantage:** lower five-year cost per verified operating hour after including acquisition, energy, updates, parts, downtime, warranty and requalification.
6. **Cross-vendor capability:** the same high-level device profile and workload constraints can be evaluated on at least two vendors and two hardware architectures without assuming feature parity.

These are engineering acceptance targets. No such comparative result has been measured in this snapshot.

## 9. Procurement and experiment rule

Before recommending or buying a specific product:

- Record country/region, exact SKU/MTM/board revision and purchase-channel availability.
- Obtain official service manual, firmware/update policy, supported OS/kernel/driver matrix, spare-parts and support-period documentation.
- Identify the security root and attestation capabilities actually provided (TPM/DICE/secure element, firmware measurements, endorsement and recovery).
- Map every candidate to frozen Symthaea tasks and actual backend availability.
- For field devices, obtain the test reports and methods behind IP/MIL claims; a datasheet summary is not a test report.
- Compare price, VAT/import costs, warranty, power, service contracts and parts availability for the same market.
- Prefer loan/evaluation units or existing lab equipment before committing scarce capital.

No exact purchase recommendation is made here; the shortlist should be regional and workload-matched.
