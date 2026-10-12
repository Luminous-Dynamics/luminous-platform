# OpenWrt One: Lab Gateway Evaluation Plan

**Status:** Planned; no hardware has been physically inspected or tested by this repository  
**Research snapshot:** 2026-10-10  
**Catalog entry:** HW-001 in ../open-hardware-inventory.md and ../../hardware/catalog.yaml  
**Tracking:** [Issue #19](https://github.com/Luminous-Dynamics/luminous-platform/issues/19)

## 1. Purpose and constraints

Evaluate the OpenWrt One as an open-source router platform for controlled lab and small-site experimentation. It is not selected as the government-network production boundary, a validated cryptographic appliance, or an approved system for protected/classified data.

The initial design deliberately does not modify the PCB. Use a traceable, commercially manufactured unit and qualify its firmware, network behavior, recovery path, sourcing and support. Only propose a derivative board after a documented requirement cannot be met by the published design or by a supportable commercial appliance.

## 2. Public evidence and source completeness

The official [hardware directory](https://one.openwrt.org/hardware/) lists:
- KiCad schematic sources including MT7981, power, PCIe/USB/GPHY, Wi-Fi and peripheral sheets.
- A PDF schematic named BPI_OpenWRT_ONE_V10_SCH_20240618-R.pdf.
- A PDF PCB package named BPI_OpenWRT_ONE_V10_PCB_20240619.pdf.
- Hardware material dated in the directory in June 2025.

The official [software source page](https://one.openwrt.org/sources/) lists source packages corresponding to released OpenWrt One software. The [OpenWrt hardware profile](https://openwrt.org/toh/openwrt/one) describes a MediaTek MT7981BA/Filogic platform with 1 GB RAM, a 2.5 GbE WAN port, a 1 GbE LAN port, Wi-Fi 6, and an M.2 slot; the page also notes that a recent batch had a physical M.2 mounting-post issue.

**Source-completeness finding:** editable schematics are listed, but the directory evidence inspected here did not establish that the preferred editable PCB layout and a complete, sourced BOM are published with the files. Treat the design as **schematic-visible, PCB-source/BOM completeness unproven**, not as a complete reproducible open-hardware manufacturing package. Record the actual board revision and correlate it with the source files before doing anything else.

Availability is not equivalent to long-term supply. The OpenWrt device database indicates availability in 2026, but the specific seller, batch, lead time, warranty and replacement strategy must be checked at procurement time.

## 3. Intended lab topology

Use an isolated lab network:
- OpenWrt One WAN port connects only to a controlled test uplink behind an independent lab perimeter.
- The LAN port connects to a managed test switch and isolated test endpoints.
- Administration occurs from a dedicated management interface/network or directly attached console; do not expose the router's management service to untrusted WAN traffic.
- No production agency traffic, real credentials, production keys, sensitive personal data, or regulated data may traverse the test router.
- Wireless operation remains disabled until radio configuration, legal bands, authentication, management isolation and coverage are separately reviewed.
- Packet captures and logs are synthetic or scrubbed. Keep firmware images, configs, results and device identifiers in a versioned test packet without secrets.

If the platform cannot provide a separate management interface on this model, use an isolated physical lab port/subnet and document the limitation. Do not create an implicit management path from the WAN.

## 4. Bring-up checklist

1. Record purchase source, date, invoice, serial/lot identifiers where present, PCB silkscreen revision, SoC/platform revision, included accessories and visible hardware deviations.
2. Capture photographs of the device, label and PCB markings. Store them with acquisition records; redact personal/order data before publication.
3. Hash the initial firmware image and record the official source URL, release, signature/checksum information, tool versions and retrieval date.
4. Read current upstream release and security advisories; choose a supported release explicitly. Do not use an image solely because it is the image shipped in the box.
5. Verify the documented recovery process and serial-console procedure on a disposable unit before configuration. Do not assume secure boot, anti-rollback, or a hardware root of trust; inventory what the actual boot chain proves.
6. Record default services, accounts, listening sockets, package list and wireless state before hardening.
7. Apply a minimal configuration, unique administrative credentials, least-privilege management access, disabled unused services, and explicit ingress/egress policy.
8. Export a redacted configuration backup. Confirm the backup omits credentials and cryptographic secrets before publication.
9. Save exact source/config hashes and all raw test outputs. Distinguish expected behavior, observed behavior, failure, and untested behavior.

## 5. Acceptance test plan

These are **planned tests, not passing results**. Each row must have an execution log, exact firmware/device revision, configuration hash, command or method, expected result, observed result, verdict and reviewer.

| ID | Test | Required outcome |
|---|---|---|
| OW1-01 | Artifact provenance | Firmware is obtained from an authoritative source; digest/signature checks and exact release are recorded. A missing check is a failure, not a warning that can be ignored. |
| OW1-02 | First boot and credentials | No shared/default credential remains enabled; no required secret appears in the published configuration or logs. |
| OW1-03 | Management exposure | Management interfaces are reachable only from the designated lab management path; none are exposed from the test WAN. |
| OW1-04 | Zone isolation | The documented allow-list works; forbidden WAN-to-management, guest-to-staff, and unapproved east-west paths fail closed. |
| OW1-05 | IPv4 and IPv6 | Filtering, routing, DNS behavior, inbound exposure and leak paths are assessed independently for both protocol families. |
| OW1-06 | VLAN and wired port behavior | Test the documented VLAN/port contract; unsupported trunking or segmentation claims are removed from the product profile. |
| OW1-07 | Wireless | If enabled for a separate test, verify encryption/authentication, management isolation, guest separation, update behavior and jurisdictional radio settings. Otherwise record as not tested/disabled. |
| OW1-08 | Firmware update | Update from the approved source, verify integrity, record reboot behavior, and confirm service recovery. |
| OW1-09 | Interrupted update / power | Controlled interruption does not silently bypass verification; recovery returns to a known state, or the limitation is documented and disqualifies the desired profile. |
| OW1-10 | Factory recovery | Recovery method is reproducible and restores a known image/configuration; operator steps and required tools are documented. |
| OW1-11 | Performance and thermals | Measure throughput, latency, packet loss, CPU/memory and temperature across representative packet sizes and concurrent traffic. No number is claimed before measurement. |
| OW1-12 | Failure and reboot | Link loss, DHCP/DNS failure, reboot and WAN outage have documented behavior; no stale or permissive management policy is introduced. |
| OW1-13 | Vulnerability response | Identify update channel, security-advisory path, supported-release policy, and the owner/timing expectation for applying critical fixes. |
| OW1-14 | License and bill of materials | License scope is clear; full component BOM, alternates, and availability can be verified or recorded as outstanding. |
| OW1-15 | Supply/lifecycle | Source of hardware, current availability, warranty, spares, hardware revision and replacement path are recorded. |

## 6. Disposition gates

- **Continue as lab reference** if firmware provenance, isolation, recovery, and bounded performance requirements can be demonstrated.
- **Remain lab-only** if boot-chain, update, source completeness, component provenance, performance, support or regulatory gaps remain unresolved.
- **Do not promote to a production boundary** until a customer-specific requirement set is defined and all required product, security, supply, environmental, crypto-validation and authorization gates have evidence.
- **Open-hardware package complete** only after preferred editable schematic/layout files, compatible licenses, BOM with alternatives, build/assembly instructions, firmware source/closed components, tests, limits and manufacturing evidence are available. If the PCB layout/BOM cannot be sourced, keep the gap public and do not claim a fully reproducible derivative.

## 7. Artifacts to produce when hardware is available

- Device identity/revision sheet and redacted acquisition evidence.
- Firmware artifact manifest and signed/hash-verified test inputs.
- Network diagram, allowed-flow list and redacted configuration.
- Per-test evidence packet with raw outputs and summary verdicts.
- Power/thermal/performance measurements with ambient conditions and workload.
- Source/license/BOM completion review, plus known deviations.
- Reviewer sign-off and a clear maturity-state decision.

No physical device testing has been done as part of authoring this plan.
