# Luminous Government & Regulated Infrastructure Product Line — v1

**State:** Proposed product taxonomy and research baseline; not a qualified product catalogue  
**Last reviewed:** 2026-10-10  
**Canonical machine-readable register:** [../hardware/portfolio-v1.json](../hardware/portfolio-v1.json)  
**Register schema:** [../schemas/hardware-portfolio-v1.schema.json](../schemas/hardware-portfolio-v1.schema.json)

## Decision summary

Freeze the *family boundaries* now. Do not freeze hardware SKUs, advertise a complete BOM, or imply agency authorization until exact models, source revisions, supply, support and qualification evidence have been reviewed. The first target is resilient administrative, public-service, research, and regulated-enterprise IT, including appropriate unclassified government/defence IT. Classified deployments, tactical command/communications, weapons interfaces, targeting and mission-system integration are out of scope for this general-purpose RFC.

The hardware register is a research baseline—not a procurement recommendation and not a statement of certification. As of this revision, no hardware item in the register has Luminous H1–H5 qualification status. Reference-only projects and unselected capability gaps are included deliberately so omissions remain visible.

## Product families

| Family | User-facing products/capabilities | Luminous responsibility | Required integrations |
|---|---|---|---|
| **1. Secure Core** | Device identity, enrollment, policy distribution, key/credential lifecycle integration, secure boot/update/recovery policy, evidence receipts | Own policy contracts, identity/resource mapping, configuration and evidence formats; never invent cryptography | OS/platform TPM, HSM, smart cards/tokens, existing identity provider, certificate and revocation services |
| **2. Network Edge** | Branch gateway, firewall/VPN integration, managed switching, Wi-Fi, DNS/DHCP, segmentation and site health | Reproducible configuration profiles, policy checks, upgrade/recovery orchestration and validated integration matrices | OpenWrt where the exact hardware is supported; suitable network OS and vendor-supported switches/APs |
| **3. Compute & Storage** | Managed endpoints, application servers, virtualization, local services, storage, backup and optional local AI inference | NixOS compositions, machine contracts, deployment/rollback, backup/restore qualification | Exact server/endpoint SKU, kernel/driver/firmware baseline, storage and backup products |
| **4. Fleet Operations** | Asset inventory, desired-state operations, staged rollout, maintenance windows, incident/case records and auditable remote administration | Nixward/sovereign-ops policy and state transitions, signed intent, authorization receipts, exact revision tracking | Hardware management/BMC, SSH or vendor APIs behind least-privilege adapters; identity provider |
| **5. Assurance & Resilience** | Configuration evidence, logging, monitoring, incident-response workflow, vulnerability/obsolescence tracking, backup verification and offline recovery | Evidence format, verification tools, policy gates, lifecycle evidence and reproducible validation | SIEM/log system, scanner/advisory feeds, backup targets, out-of-band network and customer assessment records |
| **6. Communications & Site Infrastructure** | Voice/video integration, isolated management access, rack, UPS/PDU, cooling and environmental telemetry | Integration profiles, access policy and failure/recovery tests | Specialist appliances and devices selected for the customer’s actual operating and information-handling constraints |

These are family boundaries, not six release-ready SKUs. Each public product needs an owner, versioned API/configuration contract, supported-model matrix, lifecycle policy, upgrade/recovery policy, test suite and maturity label.

### Responsibility boundaries

- **First-party software:** Luminous Platform components, NixOS composition, Nixward and fleet policy, sovereign-boot/recovery integration, Xenia operations/network integration, and the platform’s evidence/qualification tooling where maintained here.
- **Upstream software:** OpenWrt, NixOS, OpenBMC, coreboot, network OS projects, identity services, databases, and other third-party projects. Record exact revisions, licenses, support and residual proprietary firmware; do not relabel upstream functionality as hardware designed by Luminous.
- **Open hardware/design references:** published schematics, PCB/RTL, design specifications and manufacturing files. “Open” is recorded per artifact and license—not as a blanket property of the whole device.
- **Commercial hardware we may qualify:** ordinary servers, endpoints, switches, APs, HSMs, storage, optics, UPS, and specialist devices. Full open hardware is a preference when practical, not a substitute for a product that can be sourced, serviced and supported.
- **First-party hardware design:** defer until the gap analysis identifies a requirement not served adequately by maintainable hardware, and there is a credible manufacturing, test, regulatory, support and lifecycle plan.

## Reference deployment profiles

### A. Single-site office / lab — first qualification target

Managed NixOS endpoints; a selected edge gateway; managed VLAN-capable switch and AP; local identity/DNS/DHCP or documented service dependencies; encrypted backup; basic monitoring; UPS; an independent administrative/recovery path. Default-deny inter-segment policy, a clear guest/IoT boundary, and a documented offline recovery procedure. Start here before attempting multi-site claims.

### B. Small branch or remote service point

The same baseline in a smaller footprint, with explicit WAN-loss behavior, optional carrier backup where locally available, store-and-forward operational telemetry, power-loss tests, remote recovery and spares. The branch must continue safe local operation when the central management plane is unavailable.

### C. Multi-site agency

Repeatable per-site configurations; centralized identity/policy; mutually authenticated site links; central inventory and evidence aggregation; staged and canaried upgrades; local break-glass paths; bounded outage behavior. A central console is not itself an authorization authority, and a management-plane compromise must not automatically grant unrestricted execution on every site.

### D. Datacenter / edge installation

Selected, serviceable compute and storage nodes; segmented production, storage and out-of-band networks; appropriate switch/NIC/optics matrix; backup/restore and power/cooling monitoring; hardware asset provenance; BMC access restrictions; and offline or separately controlled recovery. Do not adopt a hyperscale OCP design solely because it is open—the form factor, scale, energy, repair model and local supplier ecosystem must match the deployment.

All four are architecture targets. None becomes a qualified configuration until a versioned deployment manifest and exact hardware/software bill of materials are linked to observed test evidence.

## Hardware selection and maturity contract

The machine-readable register is the source of truth for candidates and gaps. Maturity uses these states:

- **H0 — Candidate:** source or product lead identified; unresolved blockers remain.
- **H1 — Source reviewed:** exact model/revision is pinned; design/product documentation and license/terms reviewed; BOM/source completeness assessed.
- **H2 — Buildable/procurable:** manufacturing path or supplier availability is evidenced; parts, substitutions, firmware and lead times are traceable.
- **H3 — Platform integrated:** exact configuration boots, updates, reports health, enforces intended policy and recovers in the supported platform.
- **H4 — Operationally qualified:** tests demonstrate the defined profile’s performance, security, maintenance, degradation, failure recovery and recovery-time objectives.
- **H5 — Deployment-authorized:** the *named customer/environment/profile* has the required assessment and authorization. This is not inferred from source availability or an Luminous test.
- **Reference only / gap unselected:** a standard or upstream project is a useful direction, or a capability has no selected model. It is not an offerable hardware product.

No status advances because a PR was opened, a workflow queued, a BOM guessed from a web page, or a simulation passed. Exact-head evidence must bind to source revisions and test environment. Open or incomplete findings remain blockers.

## Open-hardware package requirements

For every candidate we intend to build, modify or resell as open hardware, collect:

1. Upstream URL and immutable source revision; artifact-by-artifact license and provenance.
2. Native editable files (schematic/PCB/RTL/CAD), fabrication outputs, assembly files, parts list, approved substitutes, toolchain versions and manufacturing notes.
3. BOM completeness, current supplier quotes/stock, minimum order quantity, lead times, end-of-life and repair/spares plan.
4. Boot ROM/firmware/management-controller inventory; signing and update trust; TPM/root-of-trust/key provisioning assumptions; recovery path; proprietary binaries and their exact role.
5. Threat model, trust boundaries, radio/regulatory status where relevant, power/thermal and interface limits, and known silicon/board errata.
6. Test plan and exact-run evidence: compatibility, negative cases, fault injection, reset/power-loss behavior, update rollback, restore, and performance under supported workload.
7. Separate licence compatibility, product safety/regulatory requirements, cryptographic-module validation, operating-system compatibility, and customer authorization. They answer different questions.

A PDF schematic by itself, open RTL without an available manufactured part, a source archive without a verified BOM, and an OS image that boots are all useful partial evidence—not proof of a reproducible, qualified product.

## Current research findings and gaps

The current register links to primary-source leads and captures limitations. Highlights:

- **OpenWrt One:** official hardware materials include KiCad schematic files and board/schematic PDFs. Pin exact board revision; check recent-batch details, licenses, complete BOM, recovery/update path and actual firewall/VLAN workload before treating as anything beyond a lab/branch candidate.
- **OCP networking:** the project publishes open networking specifications and design packages, including a Minipack3 design package. Choose a concrete model, source revision, license, manufacturer, NOS and support arrangement.
- **OCP server, storage, rack/power and hardware management:** strong source/specification ecosystems; each remains a design-selection exercise, not an automatically qualified bill of materials.
- **OpenTitan:** this is now a stronger production-silicon lead: official documentation states Earl Grey is in production and the project timeline records production deployment in Chromebooks in March 2026. Luminous still needs an exact purchasable part/package, supply channel, integration fit, provisioning design, lifecycle evidence and customer-specific assurance; upstream production deployment is not Luminous qualification.
- **BeagleV-Fire:** published board-design files and schematics make it a useful lab/FPGA lead, not a default government server or certified security device.
- **Framework Laptop 13:** current Framework compatibility material for the Ryzen AI 300 Series lists NixOS as community-supported; Wi-Fi/Bluetooth work out of the box in that configuration, while fingerprint-reader setup is required. This narrows an initial candidate but does not qualify the exact machine, firmware, security policy, warranty, or South African availability. Upstream design materials are partial rather than a complete public mainboard schematic set.
- **MNT Reform:** published open-hardware designs are valuable; the Reform v0 official listing states that version is no longer sold, so a current maintained model and sourcing path must be checked.
- **LibreRouter:** the GitHub board repo is archived and points to a GitLab successor. Verify successor activity and source completeness before further consideration.
- **OpenBMC/coreboot:** software integration leads, not proof that a whole server/mainboard is open or well-supported. Verify the exact machine and binary dependencies.

The initial candidate inventory is intentionally incomplete. High-priority not-yet-selected classes include production server/endpoint SKUs; managed switches/APs; validated cryptographic module/HSM and identity tokens; backup/storage; NICs and optics; UPS/PDU/environmental monitoring; independent console/recovery; WAN/cellular hardware; and approved voice/video endpoints. Each needs a model-level sourcing and support decision.

## South African public-service policy starting points

The [official DPSA standards index](https://www.dpsa.gov.za/policy-updates/e-gov/e_government/standards/) lists several relevant primary-source entries, including Public Service Information Security Directive material (2022), the Public Service ICT Security Assessment Standard and checklist (2018), ICT Security Guidelines and ICT Service Continuity Management sub-guidelines (2017), MIOS v6, and older FOSS and MISS policy materials. This should be treated as a discovery index—not as proof that every linked item remains current, applies to every agency, or governs every procurement. Before a product profile is finalized, retrieve the actual signed documents and circulars, confirm their current status with the relevant agency/contract authority, and map the required controls to explicit acceptance evidence.

## South African procurement path (research lead, not eligibility advice)

The procurement route is part of the product plan, not an afterthought. The [SITA effective-panel listing](https://www.sita.co.za/content/sita-effective-panel-contracts) states that its listed contracts are available to government departments, parastatals and organs of state. As checked on **10 October 2026**, it lists RFA 2494-2021 (Information Security Products and Services) through 6 September 2027 and RFA 2168-2024 (server-room-related network cabling, solar and infrastructure services) through 10 October 2031. The same page lists RFA 2501-2021 for unified communications/video conferencing through **8 October 2026**, which is already past the review date and must not be treated as active without a new official confirmation. [SITA’s hardware contract page](https://www.sita.co.za/node/186) lists RFB 2009 from 20 April 2026 to 20 April 2027 and RFB 740 from 24 November 2025 to 24 November 2026; verify current documents, scope and amendments before relying on either.

The [gCommerce supplier portal](https://www.gcommerce.gov.za/Home/Supplier) describes sourcing routes for transversal contracts and public-sector procurement. A practical commercial route may be to partner with an accredited SITA supplier or systems integrator while Luminous supplies the software, platform integration, support evidence and open-hardware qualification package. This is a **hypothesis to test**, not a claim that Luminous is currently an accredited supplier, that every contract is open for new entrants, or that a given Luminous product satisfies a tender. Before selecting a channel, read the actual engagement model, technical specifications, reseller list and eligibility rules, then map each product to contract scope and customer requirements.

## Procurement and compliance profiles

Build separate requirement maps for:
- US civilian/public-sector deployments.
- US DoD or defence-adjacent **unclassified** IT.
- South African public-sector deployments.
- Vendor-neutral regulated enterprise and critical infrastructure.

For each customer, record the governing authority, contract clauses, data classification/handling constraints, jurisdictional radio/electrical rules, cryptographic-module requirements where applicable, identity/access requirements, retention and audit requirements, support/residency constraints, assessment evidence and exceptions. Do not copy an assumed US FIPS requirement into every South African deployment; confirm the actual applicable standard and procurement terms. Use the NIST CMVP registry when FIPS validation is required, and check the exact certificate scope/status—not merely the vendor or device family.

NIST SP 800-161 Rev. 1 Update 1 is a useful supply-chain risk reference for supplier, component and development/manufacturing process risk. It does not certify a product and does not replace requirements from the relevant authority.

## Immediate implementation sequence

1. Validate this catalog/schema as CI data and fail closed on malformed records, duplicate IDs, invalid links, maturity/authorization contradictions and source revision omissions.
2. Select a small office/lab bill of materials with at least one fallback per critical part; capture local supplier availability and support evidence rather than shopping from marketing pages alone.
3. Pin one endpoint, one gateway, one switch/AP, one server/storage configuration and UPS/management hardware; test the reference deployment end-to-end.
4. Create complete native-file evidence packages for the highest-value open designs where licenses and source collateral permit.
5. Add automated failure-injection and hardware-in-the-loop tests only where physical hardware behavior cannot be responsibly inferred from simulation.
6. Expand to multi-site and datacenter profiles after the first profile is repeatable.

## Primary-source reference list

- [NIST SP 800-161 Rev. 1 Update 1](https://csrc.nist.gov/pubs/sp/800/161/r1/upd1/final)
- [NIST Cryptographic Module Validation Program](https://csrc.nist.gov/Projects/cryptographic-module-validation-program/validated-modules)
- [Open Compute Project Networking](https://www.opencompute.org/projects/networking/)
- [Open Compute Project Server](https://www.opencompute.org/projects/server/)
- [Open Compute Project Storage](https://www.opencompute.org/projects/storage/)
- [Open Compute Project Rack & Power](https://www.opencompute.org/projects/rack-and-power/)
- [Open Compute Project Hardware Management](https://www.opencompute.org/projects/hardware-management/)
- [OpenWrt One hardware documentation](https://one.openwrt.org/hardware/)
- [OpenTitan documentation](https://opentitan.org/documentation/)
- [BeagleV-Fire design documentation](https://docs.beagleboard.org/boards/beaglev/fire/03-design.html)
- [Framework Laptop 13 design documentation](https://github.com/FrameworkComputer/Framework-Laptop-13)
- [MNT open hardware](https://www.mntre.com/open-hardware.html)
- [LibreRouter board repository and successor link](https://github.com/LibreRouterOrg/board)
- [OpenBMC supported machines](https://github.com/openbmc/openbmc/blob/master/meta-phosphor/docs/supported-machines.md)
- [NixOS device compatibility](https://nixos.org/devices/)
