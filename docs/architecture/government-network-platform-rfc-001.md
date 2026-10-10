# RFC 001: Luminous Government & Regulated Network Platform

**Status:** Draft architecture proposal  
**Research snapshot:** 2026-10-10  
**Tracking:** [Issue #19](https://github.com/Luminous-Dynamics/luminous-platform/issues/19)  
**Scope:** civilian government, public-sector infrastructure, regulated enterprise, and appropriate unclassified military IT.

## 1. Executive decision

Build a coherent platform family for operating secure, resilient, auditable computing and networks. Do not claim that today's software components, an open schematic, or a procurement candidate constitute a finished government or military network.

The platform should have a stable software control plane, replaceable hardware profiles, explicit trust boundaries, reproducible deployment definitions, and measurable recovery behavior. Integrate commercially maintained hardware when that is the most supportable option; use open designs where source, lifecycle, supply chain, and qualification evidence are sufficient; and create new open hardware only where a documented requirement cannot be met well by an existing design.

This proposal covers enterprise and administrative infrastructure. It does not specify classified mission networks, tactical command-and-control, weapon interfaces, targeting systems, or operational plans for combat use. Classified environments require separate customer-led system security and authorization work.

## 2. Product-line status

The current public README is a software/platform overview, not a finalized product catalog. Product names below are **proposed packaging**, not approved SKUs. They should become products only after boundaries, interfaces, supported hardware, operations model, and qualification evidence are accepted.

| Proposed offer | Customer outcome | Primary building blocks | Status |
|---|---|---|---|
| **Luminous Control** | Fleet policy, inventory, change intent, approvals, evidence, and operator workflows | Nixward, sovereign-ops (incubating), operations-event contract | Architecture/prototype; not production qualified |
| **Luminous Edge** | Managed branch/site compute and gateway host | NixOS, luminous-edge, Xenia, selected gateway/switch/AP hardware | Reference architecture needed |
| **Luminous Site** | Secure office/campus stack with managed endpoints and local services | Edge, identity, switching/Wi-Fi, DNS/DHCP/IPAM, logging, backup, power | Not implemented as a qualified bundle |
| **Luminous Core** | Datacenter/virtualization, shared services, durable data, and local AI workloads | NixOS, approved server/storage designs, PostgreSQL, artifact mirrors, observability | Component selection and availability evidence needed |
| **Luminous Endpoint** | Managed workstation/laptop lifecycle and recovery | NixOS, sovereign-boot, Nixward, disk encryption, endpoint identity | Host components exist; hardware profile not finalized |
| **Luminous Connect** | Managed enterprise messaging, conferencing, and remote collaboration | Xenia components and interoperable protocols | Must prove production operation, security, accessibility, and compatibility |
| **Luminous Vault** | Offline-capable package/cache, backup, evidence, and recovery services | Signed artifact mirror, backup/restore, key-management integration, removable-media controls | Proposed; use qualified external HSM/KMS where required |
| **Luminous Fabric** | Policy-controlled connectivity among sites and services | Routing, network segmentation, device/service identity, approved encrypted transport | Architecture target; not a substitute for certification |

Symthaea should be an advisory/analysis capability with bounded permissions and evidence-producing recommendations. It must not be the sole authority for identity, policy approval, network actuation, or security certification. Mycelix can support cross-organization attestations and coordination where its actual identity, privacy, availability, and domain contracts are qualified; it must not be treated as the local authoritative identity provider by default.

## 3. Reference architecture

Model the estate as independently recoverable sites connected through explicitly authorized trust boundaries. A small site and a large datacenter should share contracts and lifecycle control, not necessarily the same hardware.

### Planes

1. **Physical and hardware:** power, racks, cooling, cabling, optics, servers, storage, switches, routers, Wi-Fi, endpoints, console access, sensors, and spare parts.
2. **Host platform:** supported firmware/UEFI, hardware-backed device identity where available, measured/verified boot, full-disk encryption, NixOS host baseline, signed updates, and tested recovery.
3. **Network:** IPv4/IPv6, VLAN/VRF or equivalent segmentation, routed boundaries, DNS, DHCP, IP address management, time, 802.1X/NAC where justified, Wi-Fi, WAN failover, remote-access gateways, and out-of-band management isolation.
4. **Identity and policy:** workforce identity, MFA, device identity, PKI/certificate lifecycle, service identities, least-privilege roles, emergency access, revocation, and approvals. Integrate HSMs/tokens as appropriate; do not substitute an unvalidated implementation for a required validated module.
5. **Shared services:** local directory/identity integration, configuration and package mirrors, artifact registry, secrets, databases, file/object storage, collaboration, logging, monitoring, vulnerability management, and internal documentation.
6. **Operations and evidence:** Nixward, sovereign-boot, luminous-edge, the operations API and durable event/inbox/outbox contracts, append-only audit records, signed build provenance, hardware inventory, and operator-visible state.
7. **Security operations:** alerting, asset exposure, incident case management, controlled containment actions, patch orchestration, vulnerability remediation, evidence retention, and restore drills.
8. **Continuity:** redundant network/power paths where justified, offline recovery media, tested backups, configuration export, local name/time services, local software/package mirrors, spares, and documented degraded-mode operation.

Do not make local boot, DNS, identity, logging, or recovery dependent on a remote AI service. Fail closed for authorization; fail safely for availability and boot. When a required security dependency is unavailable, deny the protected operation, preserve local recovery, and record the limitation.

## 4. Standard deployment profiles

### A. Single-site office / small agency
- Two managed gateway nodes where the availability requirement justifies redundancy.
- Managed access switches, wireless access points, and separate guest/staff/management/service zones.
- Server/virtualization capacity sized from measured workloads, plus independent backup storage.
- Central identity integration with MFA; device inventory and least privilege; local DNS/DHCP/time services with documented recovery.
- UPS sized from measured load and runtime targets, tested graceful shutdown, labelled cabling, spare gateway/optics/storage, and offline break-glass procedure.
- Endpoint lifecycle, signed update channels, vulnerability monitoring, log collection, and restore verification.

### B. Multi-site agency
Everything in A plus standardized site contracts, centrally governed but locally recoverable configuration, explicitly authorized site-to-site encrypted links, WAN failover, certificate rotation, configuration drift detection, inventory reconciliation, and tested loss-of-hub behavior. Site membership must not create broad implicit trust.

### C. Remote, low-connectivity, or intermittently connected site
A reduced local service set: cached signed artifacts, locally available documentation and recovery media, durable queued telemetry with bounded storage, explicit freshness indicators, and an approved alternate communications path. Offline mode is a planned, tested mode—not an authorization bypass. Remote radio/satellite equipment and frequency questions need local regulatory review.

### D. Datacenter / edge compute
Redundant power and cooling planning, redundant network paths, out-of-band administration isolated from production, role-separated hardware management, signed firmware and recovery, redundant storage with independently tested backups, local artifact mirrors, telemetry and capacity data, hardware replacement procedures, and service-specific recovery objectives. Select exact server, NIC, switch, accelerator, storage and optics models before documenting compatibility.

### E. Disconnected lab / high-assurance evaluation
No unapproved external connectivity; offline source/artifact import with review and signatures; reproducible build inputs; removable-media controls; independent inventory and evidence capture; and clear separation between experimental hardware and approved production equipment. This is a lab pattern, not an accreditation claim.

## 5. Security and engineering invariants

- **Identity, not network location, is the trust basis.** Authenticate and authorize users, devices, services, and requested resources explicitly.
- **Default-deny between zones.** Define allowed flows as machine-readable policy and test both intended and forbidden paths.
- **Control plane is not the data plane.** Loss or compromise of the console must not silently grant access to network traffic or endpoints.
- **No ambient actuation.** Mutations require typed operations, bounded capability, current pre-state, explicit authority, and durable receipts.
- **Every build is traceable.** Pin source revisions, lock dependency resolution, record hashes/provenance, verify signatures and attestations, and retain an offline recovery path.
- **Firmware has a lifecycle.** Require documented trust roots, signed updates, rollback/recovery behavior, and an owner for vulnerability notification and patch delivery. Evaluate evidence per device.
- **Management interfaces are isolated.** BMCs, serial consoles, switch management ports, and storage management planes must not be Internet-exposed or share broad credentials with production workloads.
- **Fail closed on authority; preserve recovery.** Unknown identities, stale approvals, policy parse errors, and failed verification must never become implicit authorization.
- **Minimize data.** Avoid raw secrets, untrusted error text, sensitive payloads, and unnecessary personal data in central logs.
- **No exact-once fiction.** External side effects need idempotency and ambiguity handling; distinguish intent, send attempt, external acknowledgement, and verified outcome.
- **No implied certification.** Passing tests, open designs, FIPS algorithm use, or general NIST mapping is not the same as module validation, customer authorization, or authority to process classified data.

## 6. Compliance profiles are independent

Create separate, versioned profiles for:
- US public-sector and federal unclassified systems.
- US Department of Defense / defense-contractor unclassified environments, with the actual contract and data type determining applicability.
- South African public-sector environments.
- Vendor-neutral regulated-enterprise environments.

Each profile needs an owner, applicable policies and editions, control-to-evidence mapping, required product configurations, data residency/retention constraints, incident reporting requirements, supplier restrictions, cryptographic module constraints, accessibility needs, and explicit acceptance authority. The first three have **not** been mapped by this RFC; they are work items, not claims of compliance.

Use authoritative sources and exact versions at implementation time. For US federal profiles, useful starting points include NIST SP 800-207 (zero-trust architecture), SP 800-53 Rev. 5 (security and privacy control catalog), SP 800-193 (platform firmware resiliency), and SP 800-218 (secure software development). For cryptographic modules, consult the exact NIST CMVP certificate, security policy, module version, and operating environment. Do not apply US-only controls to South African customers without a jurisdictional mapping.

## 7. Open-hardware strategy

Use existing designs first. Prioritize transparent schematics/layout/BOM and a supportable supply path over novelty. It may be safer to use a commercially maintained network ASIC, server, HSM or power subsystem under a strong qualification process than to manufacture an unmaintainable replacement.

For each candidate, assess separate dimensions:
- Editable source: schematics, PCB layout, mechanical CAD, FPGA/RTL where applicable, and source revision.
- License: hardware design, firmware, documentation, dependencies, and brand/trademark permissions.
- Manufacturing package: complete BOM with manufacturer part numbers, approved alternates, fabrication files, assembly files, test points, calibration and bring-up instructions.
- Availability and lifecycle: current production evidence, second source, lead times, EOL policy, support, warranty and repair options.
- Firmware trust: boot chain, signing, anti-rollback, recovery, update path, debug-port controls, and closed firmware/binary blobs.
- Security boundary: exposed interfaces, DMA/IOMMU assumptions, secrets storage, physical-access assumptions, debug ports, management plane, wireless/radio behavior, and vulnerability disclosure.
- Qualification: throughput/latency, power, thermal limits, environmental requirements, interoperability, fault-injection results, and applicable certification evidence.
- Reproducibility: a third party can build from released files, record the output and document deviations.
- Provenance: component origins and supplier restrictions are customer-specific; record evidence where it exists and do not infer it from the board designer's location.

An open-hardware package is complete only when source, license, BOM, firmware, build instructions, test/acceptance plan, safety limits, supply chain, and lifecycle support are recorded. Public PDFs without editable source are useful documentation but not a complete hardware-source release.

## 8. Product qualification states

1. **Discovered:** a source or item was found; no fit claim.
2. **Reference candidate:** useful for evaluation with explicit limitations.
3. **Reproducible prototype:** exact hardware/software revisions and build instructions exist; test results are recorded.
4. **Integration qualified:** negative, interoperability, performance, recovery, security, and lifecycle tests executed and passed for the stated profile.
5. **Procurement ready:** supply, warranty, support, provenance, component lifecycle and jurisdictional requirements have been checked.
6. **Customer authorized:** the customer's designated authority accepts the exact system for the stated classification, purpose, scope and version.

A state change requires evidence artifacts and a named reviewer. A test plan is not a test result; queued CI is not a pass; a passed software test is not hardware security validation.

## 9. First engineering sequence

### Stage 1 — Requirements and inventory
- Freeze the product taxonomy and system boundaries.
- Maintain the hardware inventory and machine-readable starter catalog alongside this RFC.
- Choose pilot scale, throughput, availability, power, temperature, portability, budget and support-life assumptions.
- Define acceptance checks and evidence owners.

### Stage 2 — Pilot site definition
- Start from the [small-site reference bill of functional materials](../hardware/profiles/small-site-reference-bom.md) and the [site segmentation/allowed-flow profile](site-segmentation-profile-v1.md), then produce exact physical/logical diagrams, network-zone and allowed-flow tables, IP/DNS/DHCP/NTP plan, rack elevation, power budget, cable schedule, BOM, spares list and recovery workflow.
- Build a disposable lab and test isolation, certificate failure, gateway loss, WAN loss, stale policy, controller outage, package mirror outage, log-storage pressure and full restore.
- Select an endpoint/server profile based on current supported firmware and drivers, not merely openness.

### Stage 3 — Hardware fit and prototypes
- Compare a commercial baseline, an OpenWrt One lab gateway, an OCP/whitebox switching option, and selected server designs.
- Only design a custom PCB when a measured gap justifies it. Start with bounded peripherals (e.g. environmental sensing or isolated management accessories) before attempting secure compute or cryptographic hardware.
- Every prototype needs schematics/layout source, licensing, BOM, assembly/bring-up guide, firmware update/recovery, test points, validation report and safety caveats.

### Stage 4 — Productization and assurance
- Pin hardware and firmware revisions; qualify failure/recovery behavior; publish support matrix and known limitations.
- Map applicable controls to evidence; commission required independent lab/vendor tests and customer authorization.
- Publish only the claims supported by the evidence.

## 10. Current non-claims

This RFC does not establish a finalized SKU list, exhaustive hardware survey, selected production BOM, classified-network approval, validated cryptographic appliance, production-ready site stack, or accreditation in either the US or South Africa. These are explicit completion targets.

## References
- [NIST SP 800-207: Zero Trust Architecture](https://csrc.nist.gov/pubs/sp/800/207/final)
- [NIST SP 800-53 Rev. 5: Security and Privacy Controls](https://csrc.nist.gov/pubs/sp/800/53/r5/final)
- [NIST SP 800-193: Platform Firmware Resiliency Guidelines](https://csrc.nist.gov/pubs/sp/800/193/final)
- [NIST SP 800-218: Secure Software Development Framework](https://csrc.nist.gov/pubs/sp/800/218/final)
- [NIST Cryptographic Module Validation Program](https://csrc.nist.gov/Projects/cryptographic-module-validation-program/validated-modules)
- [Open Source Hardware Definition (OSHWA)](https://oshwa.org/definition/)
- [Open Compute Project networking](https://www.opencompute.org/projects/networking/)
