# Luminous Sovereign Device Family Architecture 0.1

**Status:** Research and architecture draft; not a hardware release specification  
**Date:** 2026-11-09  
**Tracker:** [LP-DEVICE-FAMILY-001 #349](https://github.com/Luminous-Dynamics/luminous-platform/issues/349)  
**Portfolio:** [OMSI industry profile programme #154](https://github.com/Luminous-Dynamics/luminous-platform/issues/154)  
**Related product architecture:** [Luminous Platform product architecture PR #305](https://github.com/Luminous-Dynamics/luminous-platform/pull/305)

## 1. Thesis

Luminous should develop a **sovereign device fabric**: a family of interoperable devices whose common value is not a shared logo or a proprietary cloud account, but a common set of verifiable contracts for identity, capability, software lifecycle, privacy, workload placement, repair and evidence.

The first product family is:

- **Luminous Mobile** — personal and field smartphones.
- **Luminous Book** — laptops and mobile workstations.
- **Luminous Desk** — desktops, creator systems and engineering workstations.
- **Luminous Node** — edge computers, home/office nodes, servers and compute clusters.
- **Luminous Slate** — tablets, digital notebooks and field slates.
- **Luminous Table** — shared touch surfaces, collaborative tables and wall-scale displays.
- **Luminous Vision** — XR / spatial-computing headsets and companion compute packs.

Hearth, Gateway, Edge and Commons are adjacent infrastructure products, not additional form factors in the initial endpoint family. They may reuse the same device contracts.

This is a product architecture, not a claim that Luminous currently manufactures, sells, certifies or qualifies any of these devices. The v0.1 strategy is to qualify existing OEM/ODM hardware, compose upstream software and create reference designs. Custom boards, radios, optics and silicon require a measured gap that existing suppliers cannot satisfy.

## 2. Product and tier model

Consumer, Pro, Enterprise and Field are **profiles over a family**, not four independent hardware architectures. A family may not need all four profiles. For example, a shared Luminous Table is primarily a Pro / Enterprise / Commons surface; Luminous Mobile may have all four profiles.

| Profile | Primary optimization | Required characteristics |
|---|---|---|
| Consumer | Affordable, dependable personal use | Local-first core functions, privacy controls, accessible service, low idle power, clear support lifecycle |
| Pro | Sustained creation, engineering and development | More memory/compute, high-speed I/O, upgrade options, predictable thermals, multiple displays/peripherals |
| Enterprise | Manageable, auditable fleet operation | Fleet provisioning, hardware identity, supported firmware path, role separation, recovery, lifecycle transparency |
| Field | Low downtime in difficult environments | Replaceable wear parts, glove-compatible controls where needed, battery strategy, environmental tailoring, offline operation and field repair |

Do not withhold essential privacy, security, accessibility, repair documentation or local core functionality from Consumer products. Enterprise differences should be fleet integration, support response, lifecycle options and policy controls—not the right to have a secure device.

### 2.1 Form-factor reference matrix

| Family | Initial platform strategy | Design priorities | Explicit boundary |
|---|---|---|---|
| Mobile | ARM64 Android/AOSP on an exact, supported OEM platform; begin with software integration and device qualification | Battery, camera/microphone privacy controls, modem support, repairable screen/battery/port where feasible, long update support | Do not promise a custom phone, radio stack or bootloader unlock across vendors |
| Book | x86-64 and ARM64 reference laptops with Linux/NixOS support on exact models | Replaceable battery, SSD, keyboard, ports and display where feasible; sustained compute, good firmware support, repair docs | Do not assume every ARM laptop has upstream Linux drivers or an open firmware path |
| Desk | Upgradeable x86-64 or ARM64 workstation; discrete GPU optional | DIMM/SSD/PCIe serviceability, cooling, acoustic profile, expansion and measured local AI performance | Do not optimize on peak GPU TOPS without completed-workload quality and energy |
| Node | Qualified x86-64 and ARM64 server/edge platforms; RISC-V as a separately gated future profile | ECC where supported, remote management, storage resilience, network I/O, power efficiency, replaceable fans/drives/PSUs | Do not make RISC-V a baseline until software, firmware, IOMMU and server-platform support are qualified |
| Slate | ARM64 Android/AOSP first; Linux where exact hardware support exists | Pen/touch latency, display durability, battery, accessibility, serviceability and offline applications | Do not assume tablet-class devices have replaceable RAM or general-purpose expansion |
| Table | Reuse a qualified Desk/Node compute module with standard displays, touch controllers and replaceable I/O | Multi-user input, accessible reach, low-latency collaboration, spatial context, cleanability, cable/service access | Do not design a bespoke all-in-one compute/display assembly before validating the interaction and service model |
| Vision | Start from existing OpenXR-capable reference hardware; use tethered or separated compute where it materially lowers head mass/heat | Motion-to-photon latency, tracking stability, comfort, fit, optical safety, accessible controls, physical sensor privacy | OpenXR API conformance is not proof of tracking accuracy, safety, comfort or total system quality |

### 2.2 Processor portability

- **x86-64:** initial compatibility anchor for NixOS workstations, desktops and servers, including discrete accelerator options.
- **ARM64:** first-class target for mobile, tablets, low-power nodes and selected laptops/servers. Exact board support, firmware and peripherals must be checked per model.
- **RISC-V:** research and forward-compatibility target. Use ratified profiles and the RISC-V Server Platform specification as requirements references; do not claim production readiness from ISA support alone.
- **Accelerators:** CPU, integrated GPU, discrete GPU and NPU are separate execution backends. Record which backend actually ran each operation. A marketing TOPS number or API-visible NPU does not prove that Symthaea used it.

Avoid architecture lock-in in the application/runtime layer. Keep hardware-specific drivers and acceleration behind explicit, testable interfaces. Do not abstract away differences that affect timing, memory coherency, security or correctness.

## 3. Shared platform contract

Each supported device must have an exact, versioned profile for product SKU, board revision and relevant component revisions. The profile should reference existing Luminous evidence and inventory systems rather than introduce a competing device database.

Minimum capability dimensions:

1. Compute: ISA, CPU topology, GPU/NPU model, accelerator runtime and tested backend.
2. Memory: type, capacity, bandwidth where measured, upgradeability and ECC/error-reporting capability where applicable.
3. Storage: exact device/controller, endurance information when available, encryption and recovery behavior.
4. I/O: USB/Thunderbolt/PCIe/display/network interfaces and validated link modes.
5. Sensors and radios: exact modules, calibration state, permissions, firmware, regional SKU and tested driver support.
6. Power/thermal: power states, battery or PSU, thermal sensors, sustained limits, cooling and measured wall/battery consumption.
7. Firmware: boot chain, signing/update source, recovery method, version and known vendor support interval.
8. Software support: exact OS/kernel/driver versions, supported peripherals, known gaps and rollback/recovery path.
9. Serviceability: replaceable modules, required tools, repair documentation, parts source, service time and requalification requirements.
10. Evidence status: measured, vendor-declared, inferred, synthetic, simulated, untested or unknown; source, timestamp, exact subject and verifier.

Do not turn an unknown field into a default or inferred capability. Do not infer capability from a vendor family name. A device model and board revision must match the target profile exactly.

### 3.1 Layered trust model

Use the existing trust and firmware ecosystems, with separate responsibilities:

- **Hardware root / boot measurements:** TPM 2.0 where available on PC-class systems; DICE or an appropriate secure-element design on embedded systems where the supplier supports it. These are distinct implementation profiles, not one interchangeable primitive.
- **Evidence production and appraisal:** use an attestation architecture consistent with IETF RATS (Attester, Verifier, Relying Party). Attestation evidence is evaluated against endorsements, reference values, freshness and policy; a signature alone does not prove a device is uncompromised.
- **Firmware lifecycle:** prefer OEM-supported updates and upstream fwupd/LVFS where exact hardware is supported. Use TUF/Uptane-aligned metadata/recovery concepts where applicable. Never create a blanket universal flasher or bypass OEM controls.
- **Local enforcement:** boot verification, device access control, authorization and physical safety enforcement must remain local to the relevant trust boundary.
- **Mycelix:** may carry signed provenance, device lifecycle, witness and authorization evidence. It is not a universal root of trust, firmware verifier, firewall, sole IdP or online dependency for boot and basic local operation.
- **Operator authority:** owner/operator authorization is distinct from device attestation. A correctly attested device is not thereby authorized to perform a consequential action.

Optional Luminous services must not be required for a host to boot or retain basic local availability. Loss of authorization or effect-verification dependencies must not create new mutation permission. An indeterminate effect must be reconciled before another mutation. Compose the clarification in [platform issue #283](https://github.com/Luminous-Dynamics/luminous-platform/issues/283) and existing Nixward / Sovereign Boot contracts.

### 3.2 Firmware and software maintenance

Build on, rather than replace:

- Nix/NixOS and Nixward for declarative host state on supported systems.
- fwupd and LVFS for exact, vendor-supported firmware update paths.
- TUF for repository metadata roles, signature thresholds, expiry, rollback and freeze defenses.
- Uptane only where the automotive/two-repository threat model and roles apply.
- Vendor-native mobile, XR and server update mechanisms where those are the only supported path.

Every firmware adapter is qualified per vendor, model, board revision, firmware train and region. Record update source, signing, compatibility, release history, recovery path and warranty/support implications. If the device cannot be updated through a supportable path, state that limitation rather than implying the platform can fix it.

## 4. Cross-device workload fabric

The key product-level shift is from a set of isolated devices to a **user-controlled pool of cooperating devices**. A phone, laptop, workstation and local Node may cooperate, but the user remains in control of where data and work execute.

The placement decision should be based on:

- Task correctness and required runtime/backend.
- Data classification, locality and permitted destinations.
- Device identity and freshness-qualified trust evidence.
- Latency deadline and expected network quality.
- Energy budget, battery state and thermal headroom.
- Memory/accelerator capacity and current contention.
- Offline availability and cost of transfer.
- User consent and organizational policy.

A workload should describe its required capabilities and constraints; a scheduler may select a compatible device. This is a proposed contract, not a claim that a distributed scheduler is already implemented.

The follow-on integration contract is tracked in [DWF-001 #351](https://github.com/Luminous-Dynamics/luminous-platform/issues/351). V0 is deliberately restricted to pure, read-only or advisory computation. System/network/firmware mutation, payments, external messages and safety-critical actions stay outside the workload-placement plane and remain subject to their canonical authority/effect owners.

The work request describes constraints; it does not grant permission. Device capability, peer/session authentication, device posture, local authorization, task correctness and execution completion are separate claims. In particular, Xenia session identity does not prove workload identity or device integrity; Mycelix discovery does not create trust; an authorized lease is not a correctness proof; and a signed result is not proof that the task output is correct.

Placement must first apply fail-closed eligibility constraints (data class, allowed destination, exact runtime/model/backend, current evidence, memory, deadline, energy/thermal bounds). Only eligible devices are compared. Return a transparent Pareto set and reason codes rather than one opaque weighted score; user policy resolves any remaining trade-off or the scheduler abstains. Unknown capability is not a default capability.

**Privacy invariant:** no sensitive input may be offloaded merely because another device is faster. The destination must be authorized for that data class, satisfy the required trust policy, and expose a bounded execution contract. Minimize data movement; prefer derived features or task-local representations when they preserve correctness. Record whether work ran locally, remotely, on which exact device and software revision, and what data crossed the boundary.

**Failure invariant:** when a peer, network, verifier or coordinator disappears, the origin device either continues within its local capability or returns a clear degraded/unavailable result. It must not silently route sensitive work to an untrusted device or claim that an incomplete remote result succeeded.

### 4.1 Luminous subsystem responsibilities

| Subsystem | Hardware ecosystem role | Must not become |
|---|---|---|
| Symthaea | Local cognition, task planning, perception, resource-aware assistance and workload generation | Hardware authority, unsigned firmware updater or unverified benchmark oracle |
| Mycelix | Device identity/provenance/consent and cross-party evidence/authorization coordination | Sole local identity root, online boot dependency or substitute for independent attestation |
| Symtropy | Hardware telemetry interpretation, digital twins, consequence analysis, fault/recovery models | Physical truth without calibrated sensors and independent validation |
| Sol-Atlas | Spatial mapping, scene/geometry and environmental context for Table/Vision/field devices | A replacement for calibrated sensors, tracking validation or device attestation |
| Nixward / luminous-edge | Reproducible supported host configuration, local machine contracts and controlled lifecycle | Generic OEM firmware flasher or universal management suite |
| Xenia | Authenticated peer/site transport with offline and partition behavior | Trust appraisal or authorization by transport success alone |
| Sovereign Boot | Optional boot/recovery UX and state observation where qualified | A component required for host boot |

## 5. Repair, durability and lifecycle

Design for low lifetime downtime and low repair cost, not an unqualified promise that hardware cannot fail.

Preferred design practices where the form factor permits:

- Replaceable battery, display, keyboard, ports, fans, storage, speakers and external wear parts.
- Standard fasteners and documented disassembly; adhesives only where justified and repair procedures exist.
- Replaceable compute or I/O modules for Desk/Node/Table where this improves service life.
- Published component revision, compatible spare parts and end-of-support dates.
- Local recovery media and documented offline service procedure.
- Secure wipe / transfer workflow when a device changes owner or is retired.

Use existing repair and evidence ownership:

- Rugged smartphone / serviceable laptop design study: [#128](https://github.com/Luminous-Dynamics/luminous-platform/issues/128).
- Repair-aware requalification receipt: [#131](https://github.com/Luminous-Dynamics/luminous-platform/issues/131).
- Community test receipt and offline verifier: [PR #58](https://github.com/Luminous-Dynamics/luminous-platform/pull/58), [#59](https://github.com/Luminous-Dynamics/luminous-platform/issues/59), [PR #60](https://github.com/Luminous-Dynamics/luminous-platform/pull/60).

A repair that disturbs an ingress seal, antenna, sensor, display, battery or structural member may invalidate only the claims affected by that repair. Those claims remain suspended until the required retests pass. A successful repair receipt is not itself an environmental qualification.

Initial **design hypotheses**, not verified capabilities:

- Common field-replaceable modules: aim for 10 minutes or less using ordinary documented tools where the design permits.
- Key field-device spares and repair documentation: target at least 8 years, subject to supplier commitments and economic feasibility.
- Publish repair time, parts cost, availability, failure rate and all failed qualification attempts.
- Measure lifetime cost per verified operating hour, including repair, downtime, support and power.

Environmental ratings require the exact methods, configuration, sample size, conditions and results. MIL-STD-810H is a tailoring and test-method framework, not a universal durability certification or fixed pass/fail recipe. Use IEC 60529 for claimed IP ingress protection and publish the exact IP test claim.

## 6. Privacy, accessibility and user sovereignty

- Physical camera/microphone disconnects or shutters where feasible, with honest sensor-state indication.
- No raw sensor export by default. Keep sensor processing and retention local unless the user or authorized organization explicitly permits transfer.
- Local account and device recovery paths; no cloud account required for basic use.
- User export and deletion of personal data, encrypted backups and documented secure decommissioning.
- Accessibility across input modalities, contrast, font scaling, hearing/vision support and physical interaction.
- Repair documentation and spare parts as part of the product, not a discretionary premium.
- Clear distinction between open source software, open hardware design files, redistributable firmware, calibration data and manufacturing rights. A product is not “open hardware” merely because its OS is open source.

## 7. Build-versus-compose strategy

| Layer | v0.1 decision | Reconsider building only if |
|---|---|---|
| Mainboards, SoCs, modems, optics, batteries | Buy/qualify OEM/ODM components | A repeated measured requirement cannot be met by multiple suppliers |
| Mobile OS | Integrate with supported Android/AOSP platforms first | There is a sustained driver, update, certification and support plan |
| Desktop/laptop/server OS | NixOS-first where exact hardware is supported; validate other OS compatibility as needed | A specific operational gap cannot be solved by upstream contribution or adapter |
| Firmware update | OEM-native / fwupd-LVFS / TUF-aligned composition | A well-scoped adapter gap exists; never replace vendor signing or recovery |
| Device identity / receipts | Reuse platform, Mycelix and OMSI contracts | Existing contracts demonstrably cannot represent a required evidence type |
| AI acceleration | Integrate CPU/GPU/NPU backends and benchmark actual Symthaea tasks | Multiple measured profiles justify a dedicated accelerator |
| XR runtime | OpenXR and existing vendor runtimes | A measured interoperability gap justifies upstream work |
| Fleet management | Nixward, Xenia, Sovereign Ops and adapters | Existing ownership boundaries leave a specific unaddressed operator workflow |

RISC-V should be included in architecture reviews and conformance tests when hardware is available, but not promoted to a retail or enterprise target until firmware, IOMMU, OS, peripheral, virtualization and supplier-support requirements are all evidenced. The RISC-V RVA23/RVB23 profiles and Server Platform specification are starting references, not a product certification.

## 8. Staged engineering programme

### Stage 0 — Contract and baseline

- Publish this architecture and the benchmark protocol.
- Use the existing Luminous NixOS development host as an initial software/workload baseline; collect exact hardware inventory before interpreting any performance result.
- Record the existing RTX 2070 Laptop GPU as a known user-owned accelerator class only; do not infer host CPU, RAM, cooling, power or current configuration from that fact.
- Run read-only inventory and exact Symthaea workload profiling first. No device mutation or firmware flashing.

### Stage 1 — Reference workstation / Node

Use an off-the-shelf, serviceable x86-64 or ARM64 system with documented firmware support, replaceable storage and a measurable whole-system power boundary. Prefer borrowing or using existing equipment before purchase. Compare an integrated-memory AI PC against an upgradeable CPU + discrete GPU system only under matched Symthaea workloads.

### Stage 2 — Book / Mobile / Slate integration

Select one Linux-capable serviceable laptop and one supported Android device, each pinned to exact SKU/revision and firmware path. Integrate the Luminous experience without pretending to own their boot chain or mobile modem.

### Stage 3 — Edge / Field

Compose the existing #300 edge-node profile, #128 field-device design and #131 repair-requalification receipts. Validate offline behavior before remote operations. Add environmental testing only after the physical prototype and target environment are specified.

### Stage 4 — Table / Vision

Begin on existing OpenXR-capable hardware and commodity displays/touch surfaces. Measure tracking, latency, interaction quality, privacy and comfort separately. Keep Sol-Atlas spatial output explicitly qualified against physical ground truth.

### Stage 5 — Custom modules

Only after repeated benchmarks show a stable unmet requirement should Luminous commission custom carrier boards, secure elements, battery/power modules, sensor modules, compute modules or silicon. Any custom design must have complete editable source files, licensing, manufacturing files, BOM, firmware and reproducibility evidence before it is called open hardware.

### 8.1 Canonical cross-project design and workload owners

This Platform document is the **portfolio/product integration contract**. It does not replace the following existing design, workload, silicon, or system-model owners:

| Concern | Canonical owner | Platform-family role |
|---|---|---|
| HDC retrieval locality and executable host baseline | [Symthaea #7864](https://github.com/Luminous-Dynamics/symthaea/issues/7864) / [executable baseline PR #7866](https://github.com/Luminous-Dynamics/symthaea/pull/7866) | Reuse canonical exact retrieval workload, host CPU baseline, data-movement accounting and receipt semantics; DWF adds only local-vs-peer placement |
| Rust-native hardware energy measurement | [Symthaea ENG-ENERGY-MEASURE-001 #10460](https://github.com/Luminous-Dynamics/symthaea/issues/10460) | Reuse EnergyMeter/counter semantics and energy receipt; do not create another hardware energy API or derive joules from FLOPs/TDP |
| Workload memory and storage substrate | [Symthaea #7919](https://github.com/Luminous-Dynamics/symthaea/issues/7919) and [memory resilience #7515](https://github.com/Luminous-Dynamics/symthaea/issues/7515) | Consume capacity/locality and degradation envelopes; do not create a second memory manager |
| General CPU / RAM / GPU / storage architecture frontier | [Symthaea COMPUTE-ARCH-001 #7905](https://github.com/Luminous-Dynamics/symthaea/issues/7905) | Consume candidate profiles and task-conditioned Pareto evidence |
| Phone/laptop CPU-GPU/accelerator co-design | [Symthaea SOC-DESIGN-001 #8459](https://github.com/Luminous-Dynamics/symthaea/issues/8459) | Map phone and laptop family constraints onto this shared product taxonomy |
| Laptop architecture / product co-design | [Symthaea LAPTOP-ARCH-001 #8446](https://github.com/Luminous-Dynamics/symthaea/issues/8446) | Use its laptop archetypes, repairability, service-life and workload gates |
| Personal-device energy / thermal / privacy qualification | [Symthaea MOBILE-DEVICE-001 #8413](https://github.com/Luminous-Dynamics/symthaea/issues/8413) | Compose its physical-device battery, privacy, offline and workload protocol |
| Canonical HDC/CfC workload map and conventional baselines | [Symthaea NEURO-SILICON-001A #5805](https://github.com/Luminous-Dynamics/symthaea/issues/5805) | Reuse canonical semantics, fixtures and CPU/GPU/FPGA baselines |
| HDC RTL semantic equivalence | [Symthaea #5808](https://github.com/Luminous-Dynamics/symthaea/issues/5808) | Do not create a second HDC algebra or accelerator oracle |
| Semantic GPU execution | [Symthaea GPU Fabric #6968](https://github.com/Luminous-Dynamics/symthaea/issues/6968) and [backend evidence #6991](https://github.com/Luminous-Dynamics/symthaea/issues/6991) | Consume backend identity and actual-execution receipts; no acceleration claims from API selection |
| Cross-paradigm compute comparison | [Symthaea QARCH-019 #9236](https://github.com/Luminous-Dynamics/symthaea/issues/9236) | Keep CPU/GPU/NPU/FPGA/neuromorphic/CIM/photonic/quantum families task-conditioned and on separate frontiers |
| Embedded and tiny inference standard track | [Symthaea NAS-MLPERF-001 #9647](https://github.com/Luminous-Dynamics/symthaea/issues/9647) | Reuse MLPerf Tiny v1.4 Closed/Open tracks; do not duplicate its harness or conflate its divisions |
| Laptop power/thermal twin | [Symtropy LAPTOP-TWIN-001 #2697](https://github.com/Luminous-Dynamics/symtropy/issues/2697) | Use Symtropy's calibrated, held-out model residuals and NOT_QUALIFIED behavior |
| Phone/laptop system design-space exploration | [Symtropy SOC-DSE-001 #2706](https://github.com/Luminous-Dynamics/symtropy/issues/2706) | Consume system-level Pareto results; do not create a parallel optimizer |
| Attested hardware/workload admission | [Mycelix SEC-ENCLAVE-003 #3972](https://github.com/Luminous-Dynamics/mycelix/issues/3972) and physical TPM profile [#3992](https://github.com/Luminous-Dynamics/mycelix/issues/3992) | Consume independently appraised hardware/workload evidence; Mycelix identity does not manufacture boot or workload truth |
| Peer session identity and capabilities | [Xenia host endpoint attestation #347](https://github.com/Luminous-Dynamics/xenia-peer/issues/347) and [session capability #449](https://github.com/Luminous-Dynamics/xenia-peer/issues/449) | Consume exact session identity/scope; session authentication is not workload identity, device posture or task correctness |
| Cross-device workload placement | [Platform DWF-001 #351](https://github.com/Luminous-Dynamics/luminous-platform/issues/351) | Define product-level task envelope, eligibility/Pareto policy and placement explanation only |
| Computer manufacturing/BOM closure and successor frontier | [Symthaea #5796](https://github.com/Luminous-Dynamics/symthaea/issues/5796), [#5798](https://github.com/Luminous-Dynamics/symthaea/issues/5798), [#7698](https://github.com/Luminous-Dynamics/symthaea/issues/7698) | Map supply, repairability and lifecycle constraints to sellable SKUs |

LEX-NET #1149 is a local lease profile for recognized foreign principals, not a device-compute permission. It is a semantic example only unless Mycelix's owner separately qualifies an exact device-use profile. Hardware/workload admission should compose Mycelix [SEC-ENCLAVE-003 #3972](https://github.com/Luminous-Dynamics/mycelix/issues/3972) and physical TPM profile [#3992](https://github.com/Luminous-Dynamics/mycelix/issues/3992), with Xenia session evidence and the relevant local authority contract.

The cross-paradigm portfolio is important: CPU, GPU, NPU, FPGA, HDC-native, neuromorphic, compute-in-memory, photonic and quantum candidates answer different physical/workload questions. The Platform must not imply that one architecture wins every workload. Candidate selection is task-conditioned, constraint-aware and allowed to abstain when evidence is incomplete.

## 9. Evidence and maturity language

Every claim must be labelled at the level actually achieved:

1. Conceptual — problem or design hypothesis.
2. Specified — versioned contract and test plan exist.
3. Implemented — code or physical design exists.
4. Structurally checked — static/schema/consistency validation completed.
5. CI-tested — exact head and required automated tests pass.
6. Bench-measured — actual target hardware and instruments produced a reproducible result.
7. Physically qualified — exact sample and environmental/functional procedure passed.
8. Independently reproduced — a separate party or independently implemented oracle reproduced the result.
9. Field-validated — declared operating environment and duration observed with all failures retained.

These are Luminous evidence stages, not external certification. An open design, passing VM, signed receipt, vendor statement or queued CI run does not qualify a physical device.

## 10. External reference points

- [NIST IR 8259 Rev. 1 (April 2026)](https://csrc.nist.gov/pubs/ir/8259/r1/final) — foundational activities for IoT product manufacturers.
- [IETF RFC 9334](https://www.rfc-editor.org/rfc/rfc9334.html) — architecture separating Attester, Verifier and Relying Party.
- [Trusted Computing Group DICE](https://www.trustedcomputinggroup.org/technologies/dice) — layered device identity and measured-boot evidence for constrained and non-TPM systems.
- [RISC-V specifications](https://docs.riscv.org/) — RVA23, RVB23, IOMMU and Server Platform references.
- [Open Compute Project Server](https://www.opencompute.org/projects/server/) — open server mechanical/electrical interfaces and lifecycle design references.
- [fwupd / LVFS](https://fwupd.org/) — established Linux firmware update ecosystem; exact model support must be checked.
- [TUF](https://theupdateframework.io/) and [Uptane](https://uptane.org/) — established software/firmware update trust frameworks.
- [OpenXR Conformance Test Suite](https://github.com/KhronosGroup/OpenXR-CTS) — runtime API conformance tests, not a complete XR performance or safety assessment.
- [MIL-STD-810 document status and scope](https://quicksearch.dla.mil/qaDocDetails.aspx?ident_number=35978) — environmental tailoring framework; tests must match the intended lifecycle.

## 10.1 Industry reference snapshot

The named 2026 comparator set and product-specific claims are collected separately in [Device Industry Reference Snapshot 2026-11](./device-industry-reference-snapshot-2026-11.md). It covers Fairphone Gen. 6, Framework Laptop 13 Pro, Lenovo ThinkPad P16s Gen 5, Panasonic TOUGHBOOK G2, AMD EPYC 9005 / Dell PowerEdge R7725 and HTC VIVE Focus Vision.

Those products establish existing industry capability floors. Luminous must not claim repairability, ruggedness, long software support, enterprise management or spatial computing as novel without a matched test. Use the comparator snapshot as a dated procurement/experiment guide, not as a static ranking or an assertion that vendor-reported figures have been independently reproduced.

## 11. Open questions

- Which existing public Luminous OS/mobile experience is suitable for the first Android integration?
- Which exact laptop and mini-server SKUs have the strongest combination of Linux support, firmware update path, parts availability and price in our initial procurement region?
- Which Symthaea workloads actually benefit from GPU/NPU acceleration, and which remain memory/CPU-bound?
- Which local/remote workload classes can be safely moved without exposing source data?
- Which device classes justify open reference hardware versus qualifying a commercial OEM?
- What customer evidence supports the proposed Consumer/Pro/Enterprise/Field segmentation and support pricing?

The next decision should be made from exact SKU evidence, measured workloads and customer interviews—not from brand preference or a speculative custom-silicon roadmap.
