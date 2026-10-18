# Artifact-Level Open Hardware and Repairable Connectivity Gateway v1

**Status:** draft research ledger and qualification method; no candidate is qualified or claimed fully open.  
**Issue:** https://github.com/Luminous-Dynamics/luminous-platform/issues/71  
**Research snapshot:** 18 October 2026.  
**Related:** open community connectivity PR #62; provider-neutral broker PR #64; outage-resilience PR #68.

## 1. Paradigm shift: openness is a chain of evidence, not a sticker

A router running OpenWrt is not automatically open hardware. An available schematic is not enough either. A viable community device needs evidence across the full lifecycle: the exact board revision, design files and licences, buildable software, boot chain, radio drivers/firmware/calibration, regional compliance, update/rollback, repair and replacement.

This RFC introduces artifact-level statuses instead of a single open/closed label. Each artifact must be classified as source-verified, file-available-but-license-unverified, partially open, firmware/blob-dependent, closed, unknown, or not applicable. The overall device stays a design candidate until critical dimensions have independent evidence and the device has been tested on the exact hardware/firmware revision.

The near-term goal is **replaceability and truthful disclosure first**: make the gateway a commodity, replaceable IP/WAN appliance and keep radio/access-point, local service node, power, and upstream terminal separable where operationally practical. We can use a partially open supported device for a controlled pilot while still working toward more-open hardware, but must not label the assembly fully open.

## 2. Current research candidates

### Candidate A — GL.iNet GL-MT6000 (Flint 2)

OpenWrt's device page lists the GL-MT6000 with upstream support and substantial gateway resources: MediaTek Filogic 830, 1 GB RAM, 8 GB eMMC, two 2.5 GbE ports and four 1 GbE LAN ports, plus dual-band 4x4 Wi-Fi 6. Its documented recovery options include U-Boot TFTP/serial and web recovery.

However, the vendor's firmware page explicitly describes a proprietary MediaTek SDK-based firmware path for better user experience because of stability/performance issues with open-source drivers, while also offering a synchronized firmware stream using native OpenWrt/open-source drivers. This is exactly why driver/firmware choice must be pinned and tested separately. This device is a **replaceable gateway candidate with a mixed software openness profile**, not fully open hardware.

Sources:
- OpenWrt hardware page: https://openwrt.org/toh/gl.inet/gl-mt6000
- OpenWrt Table of Hardware data view: https://toh.openwrt.org/?model=mt6000&view=normal
- GL.iNet official firmware page (lists stable/beta streams and driver distinction): https://dl.gl-inet.com/router/mt6000/openwrt25
- GL.iNet device user guide: https://docs.gl-inet.com/router/en/4/user_guide/gl-mt6000/

Do not buy or flash it solely from this RFC. Confirm current price and regional availability, model/revision, type-approval, image filename and SHA-256, install/recovery steps, selected Wi-Fi driver path, package support, and exact supported OpenWrt release before the qualification run.

### Candidate B — Banana Pi BPI-R4

Vendor documentation links BPI-R4 kernel/BSP source repositories and identifies a V1.3 hardware-file set. This makes it a useful **hardware-file and custom-board candidate** to investigate. It does not establish that the full board design is under an OSHWA-recognized licence, that every board file is redistributable/remixable, that the board can be built reproducibly, that all radio paths are open, or that the intended firmware/revision is supported upstream.

Sources:
- Banana Pi BPI-R4 docs: https://docs.banana-pi.org/en/BPI-R4/BananaPi_BPI-R4
- Vendor BSP repository: https://github.com/BPI-SINOVOIP/BPI-R4-bsp
- Vendor OpenWrt BSP repository (documents BPI-R4 / BE13500 card support for its named branch): https://github.com/BPI-SINOVOIP/BPI-R4-MT76-OPENWRT-V21.02

The linked BSP is a vendor-specific branch, not evidence of support by the current upstream OpenWrt release. Exact licence terms for hardware files, blobs, radio calibration, regulatory data and boot chain must be reviewed.

### Role separation for the first site

| Role | First implementation rule | Openness/qualification requirement |
|---|---|---|
| WAN gateway | A replaceable OpenWrt-capable gateway; the GL-MT6000 is a candidate for tests, not a selection | Exact upstream firmware/driver branch, hashes, recovery, WAN failover, VLAN/firewall and power tests |
| Wi-Fi access point | Prefer a separately replaceable AP if this improves fault isolation; exact model not yet selected | Exact radio revision, driver/firmware/calibration, local regulatory domain and tested AP/client matrix |
| Local service/cache node | Commodity low-power compute with standard Ethernet/storage, and an exportable local-service image | Software reproducibility, storage endurance, boot/recovery, measured idle/peak power |
| Power | Measured DC/PoE/UPS subsystem selected for site load | Input/output safety, protection, battery lifecycle, runtime under measured load, safe recovery |
| Satellite/fibre/cellular WAN | A separate IP/WAN adapter | Exact kit/plan/provider terms, electrical/cable interface, permission and regional authorization; terminal itself is not claimed open |
| Mount/enclosure | Repairable, weather/heat/surge-aware installation | Material files/licence if custom, IP/thermal evidence, safe mounting and service procedure |

The first site should avoid integrating custom RF and a custom motherboard at once. Separate the questions of (a) networking works, (b) firmware is maintainable, (c) radio stack is sufficiently open, (d) the assembly is safe/repairable, and (e) public deployment is authorized.

## 3. The machine-readable ledger

The checked-in profile is at `profiles/open-hardware-connectivity-candidate-ledger-v1.json`; its strict shape contract is at `schemas/open-hardware-connectivity-candidate-ledger-v1.schema.json`.

- The overall profile is fixed to `draft_research_only`.
- Every candidate's `overall_open_hardware_claim` is fixed to false.
- A device cannot be described as fully open based only on firmware availability or design-file links.
- Unknown/closed/blob-dependent hardware dimensions remain visible.
- A candidate cannot be self-declared field-qualified; the ledger contains no field-qualified status.
- Exact model/revision, firmware hash, region/type-approval and recovery evidence are required before a device-specific bench receipt can be considered.

This schema validates the shape of the ledger, not the truth of a vendor statement or the legal status of a licence. Source claims must be rechecked before purchase. To promote a future candidate, create a new reviewed profile revision; do not edit the old research snapshot into a pass.

## 4. Artifact-level review dimensions

For the exact item and revision, record:

1. **Design source:** schematics, PCB layout, CAD, BOM, Gerbers/production outputs, enclosure and mounting files; source URL/digest; exact hardware revision.
2. **Licensing:** licence text for each design file, software component and redistribution/modification right; do not assume one project licence covers every file or radio blob.
3. **Firmware path:** bootloader, kernel, DTS, packages, build recipe, pinned commit, reproducible image result, binary blobs and firmware/calibration provenance.
4. **Radio path:** chipset/revision, in-kernel driver, required firmware, calibration source, DFS/regulatory domain behaviour, antenna constraints and regional approval. “OpenWrt supports the device” is not a radio-openness verdict.
5. **Recovery:** factory image/source, serial/U-Boot/recovery procedure, update rollback, configuration restore, power-loss behaviour and a tested clean reinstall.
6. **Supply chain:** component source/status, availability, replacement alternatives, lead time, declared end-of-life risks and repair documentation.
7. **Power/thermal:** wall/DC draw at idle/typical/peak, PoE/conversion losses, enclosure and ambient assumptions, thermal throttling and controlled recovery after power interruption.
8. **Security/operations:** update authenticity, supported release cadence, vulnerability response, management isolation, secrets handling, logs/privacy and configuration export.
9. **Legal gates:** radio type approval, spectrum as applicable, electrical/site safety, upstream terms, ISP sharing/resale and local authority/legal guidance.
10. **Field evidence:** site token, exact firmware hash, configuration hash, raw test logs and failure denominators. Synthetic/simulated tests cannot establish physical RF coverage.

## 5. Proposed qualification ladder

- **H0 / Inventory only:** source links and exact model/revision recorded; unknowns remain visible.
- **H1 / Artifact reviewed:** licence and file availability reviewed for the stated scope; hardware/firmware/driver statuses are explicit.
- **H2 / Reproducible software build:** pinned firmware build and artifact hashes verified; this does not imply all blobs are open or radio is qualified.
- **H3 / Isolated bench qualified for named functions:** WAN failover, VLAN/firewall, recovery, thermals/power and named radio behaviours pass with raw receipts.
- **H4 / Named-site field evidence:** named local site, exact firmware/region/device population, deployment approvals, power/weather/outage conditions and denominators documented.
- **H5 / Independently reviewed deployment profile:** a separate reviewer reproduces the relevant artifact/receipt checks and confirms qualification scope. This is not blanket certification of all device functions or permission to operate/resell.

These are proposed evidence levels for this connectivity workstream, not a substitute for existing formal standards or a certification mark. Do not jump levels based on one CI workflow or successful boot.

## 6. Benchmark targets (proposals, not results)

- 100% of critical BOM items have exact manufacturer/model/revision and replacement status; unknown is explicitly recorded.
- 100% of declared open design artefacts have the actual licence file and immutable source/revision evidence.
- 100% of firmware tested in bench/field receipts has an image SHA-256, build/source revision and recovery procedure.
- Zero claims of fully open hardware while any critical schematic/PCB/firmware/driver/calibration/licence dimension is unknown, closed or blob-dependent.
- Clean recovery from a deliberately bad update on 100% of predeclared test attempts, without bypassing signature/validation gates.
- Measurements of idle/typical/peak watts and Wh per successful useful task before any power-efficiency claim.
- At least 100 WAN-failure trials and a 24-hour local-only service test for the named gateway profile before a pilot performance claim.
- Every supported device/OS/AP/provider combination has a named evidence row; no generalization to a different revision.
- Zero public receipts containing credentials, private keys, packet/message payloads or subscriber identity.
- Zero unsupported claims that Starlink terminal hardware or firmware is open, or that technical Ethernet/IP compatibility grants provider sharing/resale permission.

## 7. Starlink and WAN boundary

Where the exact Starlink kit supports an IP/Ethernet interface, integrate it as a replaceable WAN adapter behind the gateway and test the exact cabling, power, bypass/bridge mode, route health and recovery. Do not depend on undocumented private APIs. Do not describe the terminal as open hardware, and do not infer permission to share/resell service from connectivity working. Provider plan terms, local authorizations and ICASA requirements remain separate deployment gates.

## 8. South African regulatory/equipment boundary

A public hardware ledger is not a type-approval certificate, frequency authorization, ISP resale licence or installation approval. Check the exact model/revision and radio configuration against relevant ICASA equipment/type-approval and spectrum/service rules, upstream ISP terms, electrical and mounting safety, and privacy requirements before public deployment. If the region-specific status is unknown, record `not_verified` and keep the deployment blocked.

## 9. Next steps

1. Select one controlled test unit only after verifying model/revision and source/recovery instructions.
2. Archive the firmware image, SHA-256, build/config revision and clean-install/rollback evidence.
3. Run VLAN/firewall, two-WAN failure, reboot recovery, thermal and power measurements with no production traffic.
4. Review the AP radio/driver/firmware/licence separately; do not infer AP openness from gateway openness.
5. Obtain provider and regulatory answers before any public sharing/relay trial.
6. Compare a more open board design against a supported commodity unit only after the commodity baseline provides real throughput, reliability, support cost and power measurements.
