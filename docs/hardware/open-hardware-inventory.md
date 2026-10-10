# Open Hardware Inventory and Qualification Ledger

**Research snapshot:** 2026-10-10  
**Status:** Initial survey; deliberately not exhaustive  
**Owner / tracker:** [Issue #19](https://github.com/Luminous-Dynamics/luminous-platform/issues/19)  
**Related architecture:** [RFC 001: Government & Regulated Network Platform](../architecture/government-network-platform-rfc-001.md)

## Reading this ledger

A candidate is not a recommendation until its exact revision, availability, license, component provenance, firmware, security behavior, lifecycle, support and test evidence have been reviewed. The maturity fields below describe public research—not whether a particular government or defense buyer may procure or deploy it.

Distinguish **open design**, **open software**, **obtainable hardware**, **security qualified**, **procurement ready**, and **customer authorized**. These are independent attributes, not one green checkbox.

## 1. Publicly evidenced candidates

| ID | Domain | Candidate | Evidence found | Proposed use | Current maturity | Main gaps / constraints |
|---|---|---|---|---|---|---|
| HW-001 | Site / lab gateway | **OpenWrt One** | Official hardware index exposes KiCad schematic files and PDF schematic/PCB documentation; official source-download page publishes source corresponding to shipped software. | Lab gateway, home/community network, controlled branch-site evaluation. | **Reference candidate** | Small embedded platform; not a datacenter switch or presumptively approved cryptographic gateway. Verify exact board revision, full BOM/alternates, supplier lead time, secure boot/recovery, firmware provenance, radio regulatory fit, throughput, and required cryptographic approvals. |
| HW-002 | Datacenter switching | **Open Compute Project networking designs** | OCP Networking describes open/disaggregated hardware and software work, with multiple design/specification candidates and an ONIE ecosystem. | Evaluate exact whitebox switch designs for campus core/leaf-spine networking. | **Design ecosystem; exact model not selected** | OCP is a portfolio, not one device. Select exact design, chip/ASIC, board, license, firmware, ONIE/OS compatibility, optics, power, cooling, and current source. Do not assume ASIC RTL or firmware is open. |
| HW-003 | Servers / chassis | **OCP server projects and published design packages** | OCP Server covers chassis, sleds, peripherals, interfaces, manageability and testing; public projects such as Project Olympus expose specifications and hardware-collateral organization. | Use for mechanical/electrical interoperability and server-design reference. | **Reference ecosystem; exact current platform not selected** | Some published designs may be historical. Pick exact revision and check editable files, licenses, BOM completeness, current processors/firmware, assembly process, parts availability and support life. |
| HW-004 | Hardware root of trust | **OpenTitan** | Open silicon root-of-trust RTL, hardware/software and verification collateral are public under Apache 2.0 except where noted; the project has commercial silicon deployments. | Architecture study, silicon/IP evaluation, future board/chip partner discussion. | **Open IP / integration candidate** | Open RTL is not a drop-in HSM or automatically available chip/module. A real product needs exact silicon supply, integration, lifecycle support, testing, and any required cryptographic/module validation. |
| HW-005 | RISC-V / FPGA development | **BeagleV-Fire** | Official documentation describes a RISC-V SoC plus FPGA board and links hardware/mechanical design files. | Lab fixture, FPGA experiments, hardware-interface prototyping. | **Development platform** | Not a generic secure server or gateway. Verify firmware/boot chain, current sourcing, toolchain, FPGA bitstream trust, thermal limits and support for the intended workload. |
| HW-006 | Open laptop | **MNT Reform family** | MNT publishes Reform-related schematics, design files and documentation; public repositories document design files and relevant licenses. | Study as an open workstation and repairable endpoint reference. | **Open design reference** | Not yet qualified as a government endpoint. Need exact model, current production/support evidence, firmware/boot chain, hardware-backed identity, wireless choices, disk encryption, drivers, endpoint management and environmental/security testing. |
| HW-007 | Community mesh / rural connectivity | **LibreRouter** | Project repositories describe an open-hardware mesh router for community networks. | Evaluate for public-interest/community connectivity and resilient rural networking. | **Research candidate** | Current board revision, component/BOM completeness, maintenance, availability, radio approvals, performance and security-update lifecycle need direct review. Do not infer suitability for a regulated boundary from the “mesh” label. |

### Authoritative references
- [OpenWrt One hardware design files](https://one.openwrt.org/hardware/)
- [OpenWrt One software source releases](https://one.openwrt.org/sources/)
- [OpenWrt One device profile](https://openwrt.org/toh/openwrt/one)
- [Open Compute Project Networking](https://www.opencompute.org/projects/networking/)
- [Open Compute Project Server](https://www.opencompute.org/projects/server/)
- [Project Olympus public hardware-design repository](https://github.com/opencomputeproject/Project_Olympus)
- [OpenTitan](https://opentitan.org/) and [source repository](https://github.com/lowrisc/opentitan)
- [BeagleV-Fire design documentation](https://docs.beagleboard.io/boards/beaglev/fire/03-design.html)
- [MNT source index](https://mntmn.com/sources.html)
- [LibreRouter repositories](https://gitlab.com/librerouter)

## 2. Unresolved hardware classes

These are tracked as gaps, not silently filled with guesses.

| ID | Hardware class | Current disposition | What resolves the gap |
|---|---|---|---|
| GAP-001 | Government-required cryptographic appliance / HSM | No specific device selected. | Customer requirement first; exact model/firmware and cryptographic module certificate/security policy; key ceremony, backup, HA, recovery and lifecycle review. |
| GAP-002 | Physical authentication token / smart card | No exact token/reader selected. | Choose identity standard; verify certification, firmware transparency where possible, reader/platform support, issuance, revocation, replacement and accessibility. |
| GAP-003 | Production router/firewall | OpenWrt One is a lab/branch reference only; no qualified boundary product selected. | Throughput/crypto benchmarks, IPv6/filtering, HA/failover, support SLA, security response, management isolation and any required certification. |
| GAP-004 | Enterprise access/aggregation switches | OCP candidates not narrowed to exact current models. | Port-speed/PoE needs, exact switch silicon/firmware, supported NOS, optics matrix, telemetry, support life, updates and failover tests. |
| GAP-005 | Wi-Fi APs and WLAN management | No production candidate selected. | Regulatory bands, 802.1X/EAP, rogue AP behavior, roaming, management segregation, firmware lifecycle and site survey. |
| GAP-006 | Supported server platform | OCP reference ecosystem only. | Exact SKU/revision, CPU/memory/NIC/storage, IOMMU and boot support, BMC isolation, firmware recovery, thermal/power profile, warranty and second-source plan. |
| GAP-007 | Storage and backup appliance | No device/drive combination qualified. | Workload and restore targets; controller/drive compatibility, encryption/key handling, snapshots/immutability, independent restore, firmware and spare sourcing. |
| GAP-008 | Managed workstation | MNT Reform is a design reference, not a selected endpoint. | Performance/driver and lifecycle needs; secure boot, firmware updates, disk encryption, hardware identity, management, docking, repair and support. |
| GAP-009 | Out-of-band management and serial/KVM | No secure appliance selected. | Separate management fabric, authentication/MFA, logging, break-glass, no Internet exposure, signed updates, supply review and recovery testing. |
| GAP-010 | Optics, fibre, copper and patching | No site-qualified bill of materials. | Speed/distance/connectors, power budgets, telemetry, interoperability, cabling certification, supplier provenance and spares. |
| GAP-011 | UPS/PDU/generator/ATS/power telemetry | No exact power stack selected. | Measured load/runtime, battery lifecycle, safe shutdown, management-port security, serviceability, local electrical codes and commissioning. |
| GAP-012 | Cooling, rack, environmental/tamper sensing | OCP provides design directions; no complete BOM/rack package is selected. | Heat/airflow modelling, rack elevation, fire/electrical requirements, sensor calibration, alarm pathway and fail-safe limits. |
| GAP-013 | Mobile voice/video and emergency communications | Xenia is a software/network capability, not a qualified hardware endpoint. | User scenarios and interoperability targets; codecs, privacy, accessibility, device management, radio/regulatory requirements and update lifecycle. |
| GAP-014 | WAN diversity / modem / satellite / radio | No jurisdiction-qualified device set selected. | Local spectrum/operator rules, coverage, carrier/service contracts, encryption/identity, environmental limits and disconnected-mode tests. |
| GAP-015 | Secure erase and hardware retirement | No standard kit/process selected. | Sanitization standard, verification evidence, key destruction and chain of custody, disposition process and customer data-handling rules. |

## 3. Design-package standard for hardware we publish

Every Luminous-authored hardware design should include, where applicable:

1. **README.md:** purpose, intended deployment, exclusions, state, revision, maintainers and known risks.
2. **LICENSES.md:** separate hardware, software/firmware, documentation, third-party IP and trademark terms.
3. **Editable native source:** schematics, PCB/CAD, mechanical files, RTL/FPGA source, pinouts/interfaces and exact revision. A PDF alone is insufficient.
4. **BOM.csv:** manufacturer part number, approved alternates, quantity, sourcing evidence, lifecycle notes and last verification date.
5. **Manufacturing files:** fabrication/assembly outputs, mechanical manufacturing instructions, revision identifiers and reproducible build notes.
6. **Firmware and boot:** supported versions, source links, closed components, signing roles, recovery, rollback and update instructions.
7. **THREAT_MODEL.md:** assets, trust boundaries, physical-access assumptions, interfaces, attack surface, secrets and failure behavior.
8. **VALIDATION.md:** environmental range, power/thermal limits, link/throughput results, negative tests, fault injection, secure update/recovery, calibration and known deviations.
9. **Supply/lifecycle record:** suppliers, lead-time snapshot, second sources, EOL plan, warranty/repair, provenance constraints where evidenced, and spare strategy.
10. **Evidence manifest:** exact source hashes, build/test commands, tool versions, raw result locations and reviewer. Missing evidence remains explicitly missing.

Adopt the [OSHWA open-hardware definition](https://oshwa.org/definition/) as the design-openness baseline: publish preferred editable source files, state the release scope, and use compatible licenses. OSHWA certification can support an open-hardware compliance claim; it is not security certification or government approval.

## 4. Selection and qualification workflow

For each candidate:
- Record exact board/device revision and upstream source commit/tag.
- Inspect the license and confirm required source files/dependencies are legally usable.
- Verify BOM completeness, manufacturer part numbers, alternates, critical component availability and second sources.
- Trace firmware and boot from reset to normal operation; document trust roots, signed stages, mutable configuration, debug access, recovery and rollback.
- Define measurable acceptance requirements before testing: throughput, latency, packet loss, power, temperature, boot/recovery, link failover, management isolation, update behavior and workload.
- Test expected and forbidden flows, identity/PKI failure, stale/replayed operator authority, external-service loss, power interruption and full restore.
- Review vulnerability response, support period, replacement/spares, supplier restrictions, and retirement/sanitization.
- Map exact jurisdictional security/regulatory requirements. Use exact certificates and approved configurations where required.
- Advance maturity only with evidence for the exact revision and a named reviewer.

## 5. Priority order

1. **First pilot:** a small office/site profile with supported commercial server, gateway, switch, AP, UPS, workstation and backup hardware. OpenWrt One and BeagleV-Fire can be controlled lab candidates, not assumptions for the production boundary.
2. **Second:** select a managed switch/NOS combination and one server platform with current supply/support evidence. Compare OCP designs with supported commercial alternatives.
3. **Third:** select identity tokens/HSM only after the jurisdiction and cryptographic requirements are stated.
4. **Fourth:** specify rack, fibre/copper, power/cooling, remote console and spares with a complete BOM.
5. **Fifth:** pursue a custom PCB only where evaluation identifies a durable unmet requirement. Prefer peripheral controllers and instrumentation before custom secure compute or cryptographic modules.
6. **Every stage:** publish only qualified claims, preserve exact build/test evidence, and version the software/hardware compatibility matrix.

## 6. Research limits

This is a source-backed initial survey, not an exhaustive global hardware discovery process. It does not include vendor quotes, physical inspections, component-level provenance verification, measured benchmarks, lab results, export/import analysis, or customer-specific accreditation. Refresh source, model, revision and availability before purchase or deployment decisions.
