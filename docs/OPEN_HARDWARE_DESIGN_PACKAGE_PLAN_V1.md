# Open-Hardware Design Package Plan v1

**State:** source-intake plan; no design package is complete or qualified  
**Reviewed:** 2026-10-10  
**Candidate register:** [hardware/portfolio-v1.json](../hardware/portfolio-v1.json)

## Decision

Reuse and qualify existing hardware before designing a new board. For this platform, separate the following outcomes:

1. **Source is published** — an upstream page or repository indicates that artifacts exist.
2. **Source revision is pinned** — exact immutable revision, file manifest and license/provenance have been recorded.
3. **Package is complete** — native design artifacts, fabrication/assembly package, BOM, substitutions, toolchain and firmware dependencies are accounted for.
4. **Build or supply is repeatable** — a physical source unit can be procured/manufactured and assembled using a documented process.
5. **Platform integration is tested** — exact hardware/firmware/software combination is reproducible and recoverable.
6. **Operational qualification passes** — physical behavior, failure modes, servicing, support and profile-specific security tests pass.
7. **Deployment authorization exists** — the customer or governing authority approves the named configuration and environment.

A published schematic or KiCad project does not automatically pass stages 2–7. A production chip does not mean a full board design exists. A product price does not mean stock, warranty or procurement eligibility is verified.

## Initial source-intake shortlist

| Priority | Candidate | What the sources establish today | What is still unproved |
|---|---|---|---|
| P0 — reuse first | **OpenWrt One 1** | Official hardware/source pages expose hardware design material, including KiCad schematic files and board/schematic PDFs; the upstream Techdata page names the product and firmware-support context. | Exact board/production-lot revision; per-file license; complete BOM and approved substitutes; editable PCB/fab/assembly package; current South African source/stock; recovery, throughput and lifecycle. First goal should be to qualify an off-the-shelf unit, not recreate its PCB. |
| P1 — open endpoint design | **MNT Reform Next** | The vendor provides a source index and states that the Reform Next docs include electronics sources (KiCad, various). A vendor update on 30 September 2026 reports first units shipped and expected larger batches in October/November. | Exact currently orderable model/module/board revision; per-file licenses; pin-pinned repository; full BOM/substitutions and assembly recipe; firmware trust/closed blobs; availability, local import/warranty/spares; NixOS signed-boot and recovery. |
| P1 — open edge compute research | **MNT Station + Reform Mainboard 3.0 + RCORE RK3588 module** | Vendor documentation describes an open-hardware, low-power modular compute bundle for homelab/edge/router experimentation. | Orderable kit/revision, complete native source and licenses, parts/assembly/BOM, exact OS/device-tree/firmware state, sustained thermal and workload evidence, NixOS support and South African servicing. |
| P2 — FPGA lab only | **BeagleV-Fire** | Official board documentation and repository expose board design material and schematics. | Immutable source/tag, artifact-level licenses, manufacturing BOM, FPGA/toolchain and bitstream provenance, current sourcing, and workload compatibility. Do not promote it as a production root of trust or general server. |
| P2 — root-of-trust integration | **OpenTitan Earl Grey** | OpenTitan publishes RTL/design and verification documentation; official project material states Earl Grey production silicon is deployed. | Exact available manufactured part/package and supplier, silicon stepping/errata, validation/certificate scope where required, provisioning design and integration package. RTL or a chip by itself is not a finished security module. |
| P3 — high-scale switch design | **OCP Meta Minipack3** | OCP lists the Minipack3 design package and related networking design sources. | Exact source revision, component license and complete PCB/BOM/fab/assembly collateral; selected manufacturer, sourceability, network OS, line-rate test suite, power/thermal/noise, support and lifecycle. |
| Hold until source reset | **LibreRouter board** | The former GitHub board repo is archived and points to a GitLab successor. | Exact maintained successor revision, source completeness, licensing, production path, radios/regulatory fit and support. Do not fork the archived design as a current baseline without that review. |

These candidates are intentionally not assigned H1 or above in the portfolio. This document records leads and the next evidence required, not a completed source audit.

## Required package directory for each design

Once a design is selected and the exact revision is pinned, create a stable package containing:

| File | Required content |
|---|---|
| `README.md` | Intended use, excluded use, exact design identity, source revision, package state and responsible maintainer |
| `source-manifest.json` | Authoritative repository URL, immutable commit/tag, each relevant source path and SHA-256 digest |
| `LICENSES.spdx.json` | Artifact-level licenses and notices, including firmware, PCB/layout, CAD, HDL, libraries and third-party logos/assets |
| `hardware-bom.csv` | Reference designator, manufacturer, MPN, quantity, package, approved source(s), lifecycle and criticality |
| `approved-substitutions.csv` | Tested substitute MPN, electrical/thermal/mechanical differences, validation scope and approver; blank means no substitutes approved |
| `fabrication/` | Native schematic/PCB/CAD/RTL sources, Gerbers/ODB++ or equivalent, drill/stack-up/impedance notes, assembly/placement files and manufacturing drawings where available |
| `firmware-manifest.json` | Boot ROM/firmware, BMC, CPLD/FPGA, wireless/radio and device firmware versions; source/binary hashes; signatures; known closed components |
| `supplier-evidence.json` | Quote date, supplier, orderable SKU, stock/lead time, minimum order, country of origin/provenance statements, warranty, spares and end-of-life/support notice |
| `manufacturing-run.md` | Tools and versions, environment, assembly process, required calibration, test points, programming process, expected yields and rejected-unit procedure |
| `threat-model.md` | Trust boundaries, exposed interfaces, boot/recovery assumptions, update path, debug/test ports, physical attack surface and mitigations |
| `qualification-plan.md` | Required negative tests, performance/thermal/power envelope, secure update/rollback, failure injection, recovery and serviceability tests |
| `qualification-receipt.json` | Exact hardware/firmware/software tuple, test fixture/revision, test-run IDs, raw artifact digests, results, gaps, assessor and scope |
| `CHANGELOG.md` | Changes to parts, PCB/RTL, firmware, manufacturing instructions, approved substitutes and qualification scope |

Use hashes for local files and immutable source revisions for repositories. A mutable `main` URL alone is not a source pin. Do not claim a “complete BOM” where only a few named major parts or a vendor’s marketing configuration are visible.

## Minimum design-package acceptance gates

- **Source gate:** immutable source commit/tag and file manifest; all artifact-level licenses reviewed; required editable files actually present.
- **Build gate:** BOM covers all components or explicitly records every unknown/NC/DNP part; component MPNs and substitutes are traceable; manufacturing outputs and tool versions are specified.
- **Supply gate:** a physical unit or manufacturing path exists for the intended region, and written supplier/manufacturer evidence gives current availability, lead time, warranty, end-of-life and spares.
- **Firmware gate:** boot chain, update source/signature, closed blobs, security fixes, fallback/rollback and recovery behavior are recorded and tested.
- **Integration gate:** exact supported NixOS/Linux kernel, device-tree/ACPI, NIC/PHY/radio/FPGA, storage and peripheral configuration is pinned.
- **Physical qualification gate:** power, thermal, RF (if present), sustained workload, interface behavior, failure injection and recovery are tested on actual hardware.
- **Operational gate:** access controls, monitoring, log retention, backup/restore, replacement/repair, warranty and support are established for the named profile.
- **Authorization gate:** jurisdictional and customer requirements are mapped and, where required, approved by the relevant authority.

No gate passes from document presence alone. Where hardware is unavailable, record a sourcing blocker rather than manufacturing a synthetic pass from simulation.

## Reproduce first, redesign second

For OpenWrt One, the first deliverable should be a qualified acquisition and integration package around a purchased exact-revision unit; do not spend effort recreating the PCB until the BOM/manufacturing package is complete and a real need to produce the board independently has been demonstrated. For MNT Reform Next, begin with upstream source intake and exact orderable configuration, then establish whether board reproduction or simply buying and maintaining the vendor hardware is practical. For OpenTitan and OCP designs, first identify a specific part/vendor/manufacturing route before attempting integration or production claims.

## Open-hardware progress state today

- **Published source leads identified:** yes.
- **Exact design revisions pinned:** no comprehensive set yet.
- **Licenses audited file-by-file:** no.
- **Complete BOM/substitution packages:** no.
- **Manufacturing runs reproduced by Luminous:** no.
- **Local supply and lifecycle verified:** no.
- **Luminous hardware-in-the-loop qualification passed:** no.

That is the honest starting line. The next source-intake pass should choose the exact OpenWrt One board revision and current MNT Reform Next source release, fetch immutable source packages, calculate local hashes, build the component/license manifests, and resolve missing BOM/firmware/supplier evidence before elevating either package.
