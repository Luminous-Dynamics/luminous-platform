# Luminous Device Family — Industry Reference Snapshot (2026-11)

**Status:** Procurement and benchmark research; illustrative comparators, not a vendor ranking  
**Date checked:** 2026-11-09  
**Parent:** [Luminous device family RFC #349](https://github.com/Luminous-Dynamics/luminous-platform/issues/349)  
**Architecture:** [device-family architecture 0.1](./luminous-device-family-architecture-0.1.md)  
**Benchmark method:** [device workload programme 0.1](../../benchmarks/luminous-device-workload-v0.1/README.md)

## Why this snapshot exists

We need to compete against actual products and mature ecosystems, not caricatures of incumbents. These examples establish capability floors and comparison candidates. They do not prove like-for-like performance, and the manufacturer figures below are not Luminous measurements.

## 1. Mobile: repairability and support already have serious competitors

**Fairphone (Gen. 6)** is a strong longevity and repairability reference. Fairphone lists 12 user-replaceable spare parts, a user-replaceable display, battery, USB-C port and back cover, spare-parts availability through 2033, software support through 2033 and a five-year warranty. The manufacturer states that several of these modules can be changed in under five minutes. Its published security description lists a Trusted Execution Environment and says there is no hardware secure element beyond the eSIM chip; this distinction matters when mapping actual trust primitives rather than treating “secure” as a generic badge.

Sources:
- https://www.fairphone.com/the-fairphone-gen-6
- https://support.fairphone.com/hc/en-us/articles/9979180437393-How-long-will-my-Fairphone-receive-software-and-security-updates
- https://www.fairphone.com/warranty

**Implication for Luminous:** A repairable phone and long software support are not novel differentiators by themselves. The opportunity is to combine those qualities with user-controlled local intelligence, offline operation, transparent sensor/data policy, independently appraised boot/device evidence, cross-device workload delegation and cryptographically linked repair/requalification records. We should compare repair cost and time against Fairphone directly on exact parts and procedures.

Do not promise an open mobile boot chain, custom modem or custom phone board until the exact OEM platform, drivers, radio certification and supplier lifecycle are known.

## 2. Laptops and workstations: repairability is entering mainstream business hardware

**Framework Laptop 13 Pro (2026)** is a high-value repairability, upgradeability and Linux-support comparator. Framework currently promotes Core Ultra Series 3, LPCAMM2 memory, modular repair/upgrades and a 20-hour battery claim. Its battery result is a vendor test under a specified media-streaming and Windows power profile; it must not be compared directly with a Luminous mixed-use or local-AI runtime unless the test is matched.

Source: https://frame.work/

**Lenovo ThinkPad P16s Gen 5** is a current professional workstation comparator. The 2026 iFixit assessment describes upgradeable LPCAMM2 memory and SSD, customer-replaceable battery, and straightforward display, keyboard and fan replacement, while noting that some ports remain soldered. Exact Intel/AMD machine type and configuration matter.

Sources:
- https://www.ifixit.com/repairability/laptop-repairability-scores
- https://www.ifixit.com/Device/Lenovo_ThinkPad_P16s_Gen_5
- https://psref.lenovo.com/

**Implication for Luminous:** Competing on “modular laptop” alone is insufficient. We need a stronger combined result: repairability, sustained local Symthaea workload performance, whole-device joules per accepted task, good Linux/firmware lifecycle, offline recovery, privacy-preserving workload placement and a validated thermal model. Use Symthaea LAPTOP-ARCH-001 #8446 and Symtropy LAPTOP-TWIN-001 #2697 as the canonical comparison owners.

## 3. Field devices: environmental and service capabilities are mature

**Panasonic TOUGHBOOK G2** is a reference for field tablet engineering and enterprise support. Panasonic lists an Intel Core Ultra 5 platform with NPU, a 1,000-nit glove-compatible display, modular expansion options, IP65, MIL-STD-810H and MIL-STD-461G testing, optional hazardous-location configuration and a three-year limited warranty. It also lists Red Hat Enterprise Linux options for this SKU. These claims are model- and configuration-specific.

Sources:
- https://connect.na.panasonic.com/toughbook/rugged-laptops-tablets/toughbook-g2
- https://connect.na.panasonic.com/toughbook/how-to-choose-rugged-laptops-tablets

**Implication for Luminous:** Do not enter field hardware with a rugged-looking case and a marketing IP claim. Start by qualifying an established rugged OEM device. Our differentiation should be offline-first workflow and cognition, authenticated networking, exact hardware/configuration identity, repair-triggered requalification and lifecycle evidence. A MIL-STD-810H reference is a tailored environmental engineering process, not a universal pass stamp; choose the methods to match declared use and publish the actual test results.

## 4. Servers and edge nodes: raw performance is a very high bar

**AMD EPYC 9005** spans up to 192 CPU cores in the top 9965 model. AMD publishes model-specific core, memory-channel, TDP and PCIe data; the part number and TDP matter more than a “9005” family label.

**Dell PowerEdge R7725** is a concrete 2U server reference supporting two EPYC 9005 processors, with up to 192 cores per processor on supported configurations. Its ecosystem includes ECC RDIMM, hot-plug components and Dell's iDRAC/Redfish-based management.

Sources:
- https://www.amd.com/en/products/processors/server/epyc/9005-series.html
- https://www.amd.com/en/partner/articles/epyc-9005-series-processors.html
- https://www.dell.com/support/product-details/en-us/product/poweredge-r7725/

**Open Compute Project Server** is a useful mechanical/electrical/process reference across open rack-compatible chassis, sleds, blades, microservers, management/debug/test interfaces, manufacturing and decommissioning.

Source: https://www.opencompute.org/projects/server/

**Implication for Luminous:** We should not set “more CPU cores” as the goal. Buy/qualify proven server platforms and measure Symthaea throughput, accepted-task joules, idle/loaded power, tail latency, failure recovery, operational serviceability and cost per verified operating hour. The edge Node product should compete on evidence-qualified autonomy for distributed sites, not on recreating Dell's server portfolio.

## 5. XR: reuse the runtime and optics ecosystem

**HTC VIVE Focus Vision** is a useful enterprise XR reference. The official VIVE Business specification lists 2448 × 2448 pixels per eye, depth and stereo passthrough sensors, eye tracking, 12 GB memory, expandable storage and a removable/swappable battery with up to two hours of battery life. The 120 Hz DisplayPort mode is labelled Beta on the business specification page; do not treat that as a universally available standalone mode.

Source: https://business.vive.com/eu/product/vive-focus-vision/

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
