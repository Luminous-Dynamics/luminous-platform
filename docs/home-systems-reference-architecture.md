# Open Home Systems Reference Architecture

**Status:** Proposal for review; architecture only. No hardware has been selected, purchased, assembled, benchmarked, or safety-qualified by this document.

**Working product family:** Hearth Node, Arc Console, Commons Compute, and Symthaea Robotics. Names are provisional.

## Purpose and thesis

Define an interoperable, sovereignty-first home-computing family using existing Luminous Dynamics projects and established external ecosystems. Avoid duplicate control planes, premature hardware commitments, and claims broader than available evidence.

Build a family of useful, independently operable systems rather than one opaque smart-home appliance:

- **Hearth Node** — local-first home computing, storage, backups, home automation, private services, and optional local inference.
- **Arc Console** — a repairable living-room PC whose primary experience is controller-first gaming and entertainment.
- **Commons Compute** — an opt-in pool of safely isolated workloads contributed by home machines and other community nodes.
- **Symthaea Robotics** — a progression from simulation and a single-joint laboratory to modular mobile robots and bounded household tasks.

All four share common host deployment, identity/evidence, operator controls, update/recovery, and trust-boundary principles. They do not need identical hardware: a quiet low-power home node and a high-power gaming machine have different budgets and thermal envelopes.

Openness must be declared component by component, and operational maturity must be stated separately from design intent.

## 1. Non-negotiable invariants

1. **Local operation first.** Core household functions continue during internet outage wherever the controlled device and chosen protocol permit it. Cloud access is optional, explicit, and removable.
2. **Owner authority.** The household owner controls enrollment, data export, compute contribution, remote access, updates, and removal. No vendor account is the sole path to basic function.
3. **Observe, propose, authorize, attempt, verify.** An observation does not establish physical state; a model proposal is not authority; a dispatched command is not a verified effect.
4. **No universal capability claims.** Compatibility is recorded per hardware configuration, software/profile version, workload, and test evidence.
5. **No required cognitive dependency for safety.** The host must boot and recover without Symthaea, network overlay, orchestration UI, or optional animation.
6. **Reversible management.** Desired configuration is declarative and versioned; risky mutations require narrow admission and post-state evidence. A failed observer or coordinator must not become a boot dependency.
7. **Least privilege and explicit isolation.** A game, plugin, downloaded model, community workload, or cognitive process receives no ambient access to host administrator sockets, secrets, home automation credentials, or raw household data.
8. **Repair before replacement.** Design for replaceable storage, accessible service points, documented parts, exportable configuration, and a supported path to install another operating system where hardware permits.
9. **Evidence before promotion.** Simulation, schema validation, CI, physical measurement, field reliability, and safety qualification are distinct evidence classes. One never silently substitutes for another.
10. **No lock-in through economics.** Contribution accounting or reputation may help coordination but must not be required to use one's own hardware. No speculative token is a prerequisite for core system function.

## 2. Layered architecture

~~~text
User / household / maintainer
        |
Local UI, controller UI, accessibility, consent and data export
        |
Product profiles: Hearth Node | Arc Console | Commons Compute | Robot
        |
Narrow capability adapters and policy-gated effect brokers
        |
Luminous Platform: NixOS profiles, isolation, updates, recovery, receipts
        |
External systems: Home Assistant, Matter, Steam/Proton, ROS 2,
other reviewed runtimes, standard file and network protocols
        |
Commodity hardware and separately qualified peripherals

Mycelix: local/peer identity links, shared rules, provenance, consent,
resource/contribution records, governance and dispute/appeal semantics

Symthaea: interpretation, planning, simulation, candidate generation,
anomaly analysis, and bounded proposals — not ambient authority

Xenia / secret services: authenticated communication and secret handling
without placing reusable credentials in model context or portable evidence
~~~

This is a responsibility map, not a claim that the integrations are complete. A provider capability must carry its exact profile and evidence status; one working feature must not imply provider-wide support.

### Existing systems are building blocks, not the whole product

The opportunity is to compose proven ecosystems rather than replace them:

- **Home Assistant already has a simple, local-first hub.** Home Assistant Green is a dedicated plug-and-play appliance; Home Assistant also supports existing x86-64 computers and other host types. Hearth Node should add a broader, repairable home-computing substrate and clear service isolation while retaining Home Assistant as an interoperable home-automation layer. Do not duplicate thousands of integrations or claim superiority at automation without comparative evidence. See the [official hardware overview](https://www.home-assistant.io/hardware/) and [Green specifications](https://www.home-assistant.io/green/).
- **Linux gaming already has compatibility machinery.** Arc should reuse Proton and Steam's per-title compatibility approach, then add transparent configuration-specific results. It should not fork a compatibility runtime before there is a measured need. See [Steam's compatibility review](https://partner.steamgames.com/doc/steamhardware/compat).
- **Modular server design already has open standards.** For larger compute nodes, evaluate Open Compute Project modular hardware and management interfaces. Treat those as candidate inputs, not requirements for every living-room case.
- **Hardware support varies by exact device and firmware.** OpenWrt explicitly distinguishes advertised open-source support from actual supported hardware and sustained development participation. Keep exact board, firmware, driver, and update status in any Luminous compatibility record; a vendor logo is not proof of support. See the [OpenWrt hardware list](https://openwrt.org/docs/techref/hardware/list).

Therefore the product claim should be narrower and more defensible than “we have an open smart-home box”: **one repair-oriented, owner-controlled home-computing foundation that composes local automation, private services, gaming, community compute and future robotics through explicit, reviewable capability profiles.**

## 3. Open-hardware policy

Record openness for each part and interface. Do not label a whole appliance “fully open” when its firmware, accelerator, radio, or other component remains a black box.

| Level | Meaning | Minimum evidence |
|---|---|---|
| O0 — exposed | External interfaces and service documentation exist | Interface inventory and service instructions |
| O1 — repairable | Replaceable wear/storage/compute parts where practical | BOM, part identifiers, repair procedure, replacement constraints |
| O2 — design-published | Mechanical/electrical design files are published with a reuse-permitting license | Versioned CAD, schematics, PCB files where applicable, BOM, design license |
| O3 — firmware-auditable | Firmware/source availability and boot/update behavior are documented | Source/build status, signing/update model, recovery route, known binary blobs |
| O4 — reproducible/forkable | A third party can reproduce the declared design/build within published limits | Reproduction procedure, tool versions, build artifacts and deviations |

These levels describe evidence, not product quality. A published design is not automatically manufacturable, safe, reliable, affordable, or repairable. Document unmodifiable firmware and upstream dependencies honestly; publish only designs the project has rights to redistribute.

This internal O0–O4 ladder is **not** a replacement for the Open Source Hardware Association (OSHWA) definition or certification. For an actual open-hardware claim, publish the design in preferred modifiable source formats, identify exactly which portions are licensed, provide a matching BOM and clear build/use documentation, and choose a license that permits the promised rights. Certification is a separate, optional claim and must not be inferred from merely publishing this repository. See [OSHWA's definition](https://oshwa.org/definition/) and [documentation checklist](https://certification.oshwa.org/process/documentation.html).

Prefer standard x86-64 or ARM64 compute, documented UEFI/boot behavior, serviceable NVMe storage, replaceable networking where possible, standard power/connectors, and conventional display/input. Apply Open Compute Project modular-hardware/server practices where relevant to larger compute nodes; do not impose data-centre hardware standards on a compact consumer console without demonstrated benefit.

Before freezing a BOM, compare candidates against explicit cost, idle power, acoustic, sustained thermal, performance, repairability, availability, driver, firmware, and security criteria. A product-specific measured profile is required; no vendor is preselected here.

## 4. Hearth Node — the home substrate

### Product boundary

The node is the local control and compute foundation. It may host storage and backups, Home Assistant, DNS/network utilities, media, personal services, local model inference, and an opt-in compute worker. Each service needs an individual workload and data-access profile; merely running on the home server grants no cross-service access.

### Required behavior

- Home automation and locally controlled devices continue through WAN loss, subject to their own protocol and power dependencies.
- Household data and backups have an understandable local retention/export/deletion policy.
- A failed inference service does not prevent manual controls or ordinary automations.
- Remote access starts disabled or is explicitly configured. Credentials stay in a secret manager and never enter prompts, logs, public manifests, or peer-shared evidence.
- The user can inspect running workloads, privileged operations, resource usage, software profile, update status, and data export routes.
- Updates have a reviewable source/profile, a rollback/recovery path, and a documented recovery route if deployment is incomplete.
- Measurements distinguish power from energy and measured values from estimates.

### Home automation integration

Build on Home Assistant and relevant protocols such as Matter, Thread, Zigbee, Z-Wave, and ESPHome where they fit; do not create an incompatible replacement ecosystem.

Target Mycelix Hearth's existing physical-effect admission path; do not create a parallel dispatch authority. The [Mycelix Hearth integration contract](home-systems-myelix-hearth-integration.md) and [Home Effect Request Envelope schema](../schemas/home-effect-envelope.schema.json) specify the Luminous-side request boundary only; they do not grant authority or imply that the open Hearth 004C/004D/004E qualification line is complete. Preserve the separation between provider observation, an authorized plan step, dispatch, and reconciled physical outcome. High-consequence device classes need stronger per-class profiles and must fail closed on unknown capability or stale authority. Model output must never be a raw command channel.

## 5. Arc Console — broad legal game compatibility

### Honest target

Build an upgradeable PC console for the broadest practical, legally supported library—not a promise to run every game ever made.

Compatibility lanes:

1. Native Linux builds.
2. Windows PC games through Proton/Wine and a declared GPU/driver profile.
3. Other launchers and stores where their terms, authentication, DRM, and technical requirements permit use.
4. Legal emulation and homebrew profiles, with users responsible for lawful game images and firmware.
5. Streaming from the owner's own PC or other devices where the owner is licensed and the upstream platform supports it.
6. Community/remote GPU rendering only for workloads and games whose terms permit the deployment and streaming arrangement.

A title may remain unavailable because of kernel-level anti-cheat, unsupported DRM, proprietary services, driver behavior, VR requirements, or performance limits. A successful launch is not equivalent to a good experience. Do not bypass DRM, account controls, anti-cheat, or platform restrictions as a compatibility strategy.

### First implementation

Start with an off-the-shelf x86-64 system and controller-first Linux gaming. Before designing a proprietary enclosure or motherboard, create a public compatibility corpus with:

- exact hardware, firmware, OS image, kernel, GPU driver, Proton/runtime, and game version;
- launch result and whether the title reaches an interactive state;
- frame-time distribution, frame rate, resolution, graphics preset, power/temperature, and crash behavior;
- controller, audio, display, suspend/resume, multiplayer, launcher, and anti-cheat outcomes;
- freshness timestamp and reproducible test instructions.

Report per-title status such as **Verified for Profile**, **Playable with Limitations**, **Unsupported for Profile**, **Untested**, or **Evidence Stale**; do not publish a universal “all games work” badge. Separate provider assertions from project-run observations.

## 6. Commons Compute — volunteered capacity with owner-controlled limits

The initial scheduler must be a local-first, explicit opt-in service, not a background process that silently rents the owner's GPU.

Each node advertises a versioned capability profile and owner-defined limits: allowed workload classes, CPU/GPU/RAM/VRAM/storage, temperature, power budget, quiet hours, metered-network policy, maximum job duration, and pause/revoke behavior. Local interactive workloads and household services take priority.

### Workload classes

- **Public reproducible:** inputs and outputs are intentionally public and the work can be independently rerun or checked.
- **Private/local-only:** household, personal, regulated, confidential, or licensed data stays on the owner's node.
- **Restricted remote:** allowed only when the owner, task owner, and licensing/security profile explicitly permit the transfer and execution.
- **High-integrity:** requires a declared validation strategy, such as deterministic recomputation, independent replicas, known-answer fixtures, or an application-specific proof. A signed receipt only proves who submitted it and what they claimed; it does not prove that computation was correct.

### Isolation and accounting requirements

- Run each job in a constrained VM or comparably strong, reviewed sandbox where practical; an OCI container alone is not a complete boundary against a hostile host/kernel or untrusted workload.
- Deny network egress by default. No home LAN, hypervisor, container/runtime, secret-store, or administrator-socket access.
- Pin runtime/artifact digests and resource limits; impose timeouts and bounded output sizes.
- Treat artifacts and node claims as untrusted; validate outputs outside the contributing worker where needed.
- Show the job owner what data leaves, the node operator what will run, and both parties how cancellation and deletion work.
- Preserve attempt, node/profile, artifact, resource-usage, result, validator, and dispute evidence without putting secrets or sensitive inputs into a public DHT.
- Fail safely when connectivity is lost. Local home services must not depend on community task execution.
- No cryptocurrency mining or open-ended paid work by default; contribution is opt-in, revocable, and transparently accounted.

Before building a scheduler, write a small fit/gap matrix against BOINC/BOINC Central and any managed-cluster runtime considered for the intended workload. BOINC is established prior art for scientific volunteer computing and documents signed application distribution, resource limits, sandboxing, and replicated result validation; it also documents residual trust risks. The first implementation should prefer an adapter or owner-controlled gateway around existing machinery if it can meet the required policies. Build a new scheduler only for demonstrated requirements the existing options cannot meet without weakening security, owner consent, locality, or result evidence.

The first prototype should submit only synthetic, public, short-lived jobs to two test nodes. Then inject malicious output, stale software, repeated jobs, partial output, disconnection, cancellation races, and resource exhaustion. Do not start by distributing private Symthaea training or personal files.

## 7. Symthaea Robotics — cognition above a bounded control plane

Symthaea can help interpret observations, propose tasks, compare candidate designs, plan within a declared capability envelope, analyze anomalies, and prepare experiments. It does not replace the robot's low-level deterministic controller, hardware interlocks, physical emergency stop, commissioning, calibration, or task-specific authorization.

### Recommended progression

1. **Simulation and fixtures:** build the task model, sensor/actuator contracts, and adversarial scenarios. Track simulation evidence separately from physical evidence.
2. **One instrumented joint:** follow Symthaea ROB-DESIGN and ROB-JOINTLAB. Measure the real actuator, power/current, position, temperature, load, timing, calibration, and sim-to-real discrepancy.
3. **Teleoperated mobile base:** add conservative indoor navigation, obstacle sensing, dock/charge, hardware stop, remote kill, and a safe manual recovery route before task autonomy.
4. **Low-risk household tasks:** enable one narrowly described task at a time, with exact environment limits, preconditions, abort conditions, and observed outcomes.
5. **Manipulation and expanded tasks:** add arms/end-effectors only after relevant joint, grasp, collision, thermal, power-loss, recovery, and human-interaction profiles qualify.

Robot software must not infer that a task succeeded merely because it sent a command or the simulator succeeded. Use an independently observable outcome and an explicit **Unknown / Needs Reconciliation** state where evidence is incomplete. Preserve an independent physical stop/power-removal path appropriate to the embodiment. Never expose general-purpose host administrator capabilities directly to the motion-planning model. Camera/microphone collection defaults must be explicit and privacy-preserving.

## 8. Responsibilities across repositories

| Project | Owns | Must not own |
|---|---|---|
| luminous-platform | This cross-product reference architecture; NixOS/workload profiles; isolation, updates, recovery, operator receipts | Universal device semantics, unbounded cognitive authority, vendor-specific product truth |
| luminous-edge | Reusable NixOS composition for qualified platform components and declared hardware/workload profiles | Silent privilege escalation or optional components required for host boot |
| Mycelix existing Hearth / ecosystem / compute / hardware work | Household/peer identity and consent semantics, capability/profile evidence, shared resource/contribution records, governance, reconciliation boundaries | Direct motor control; provider data, identity, attestation, or reputation treated as automatic authority |
| Symthaea | Cognitive proposal/planning and evidence-bounded robotics design/experiment pipeline | Direct unmediated host/device writes or physical-actuation authority |
| Xenia and secret-handling components | Authenticated transport, session protection and secret boundaries | Reusable secrets in model context or general evidence |
| External systems (Home Assistant, Matter, Steam, ROS 2, etc.) | Their native device/provider/runtimes and declared interfaces | Canonical Luminous identity, truth, consent, or qualification |

Reuse existing tracked work instead of creating a competing architecture:

- Mycelix [INT-ECOSYSTEM-000 #2871](https://github.com/Luminous-Dynamics/mycelix/issues/2871): Steam and Matter/Home Assistant integration programme.
- Mycelix [HTH-AUTO-IOT-000 #2879](https://github.com/Luminous-Dynamics/mycelix/issues/2879): Home Assistant/Matter profiles behind the existing Hearth physical-effect admission path.
- Mycelix [MYC-INT-018K/L #3241/#3243](https://github.com/Luminous-Dynamics/mycelix/issues/3243): vendor-neutral physical hardware capability and admission rubric.
- Mycelix [SEC-ENCLAVE-003 #3972](https://github.com/Luminous-Dynamics/mycelix/issues/3972): attested hardware/workload admission boundary.
- Symthaea [ROB-DESIGN-000 #4795](https://github.com/Luminous-Dynamics/symthaea/issues/4795): evidence-bounded recursive robotics design.
- Symthaea [ROB-JOINTLAB-001 #4814](https://github.com/Luminous-Dynamics/symthaea/issues/4814): first bounded instrumented physical robotics lab.
- Symthaea [ROB-MFG-001 #5775](https://github.com/Luminous-Dynamics/symthaea/issues/5775): repair/replacement-part and productive self-maintenance evidence.
- Luminous Platform [issue #38](https://github.com/Luminous-Dynamics/luminous-platform/issues/38): NixOS hosts, OCI profiles and cross-OS VM workload strategy.

These links are ownership dependencies, not claims that the work has completed or passed qualification.

## 9. Machine-readable product profiles

The architecture now has a machine-checkable contract alongside this narrative:

- [Profile schema](../schemas/home-system-profile.schema.json) — JSON Schema Draft 2020-12.
- [Positive profile fixtures](../profiles/home-systems/valid/) — Hearth Node, Arc Console, Commons Compute, and an instrumented robotics-lab design; all intentionally design-only.
- [Negative fixtures](../profiles/home-systems/invalid/) — attempts to promote an unimplemented capability, leak secrets into model context, omit the robotics physical-stop requirement, remove compute opt-in, or claim qualification without evidence.
- [Rust validator and tests](../tools/home-profile-validator/) — schema meta-validation, structural validation, exact profile-ID/subject-digest binding, and positive/negative fixture checks.
- [Contract CI workflow](../.github/workflows/home-system-profile-contract.yml) — runs Rust tests and validates fixture directories when relevant files change.

The schema and Rust checker are deliberately modest and fail-closed. The current Rust checker rejects every profile declared `qualified`, even when the JSON contains a matching digest and passing-looking receipt reference: it does not fetch or authenticate receipts, independently hash external artifacts, confirm a test took place, prove hardware identity, or certify physical behavior. A future qualification path must supply an independently reviewed verifier and authoritative evidence source before this behavior can be relaxed. No design-only fixture is allowed to carry an implementation digest, prototype, benchmark, non-zero current openness level, recovery test, or qualification record.

## 10. Delivery plan and acceptance gates

### Gate A — architecture/profile freeze

- Ratify product vocabulary, repository boundaries, openness levels, data zones and nonclaims.
- Freeze hardware capability classes and workload profiles before selecting SKUs.
- Declare user personas and target profiles (quiet/low-power, gaming, robotics, or combined) separately rather than trying to optimize one box for every dimension.

**Exit evidence:** versioned requirements, traceable decisions, profile schema and negative fixtures. No purchase approval follows automatically.

### Gate B — useful Hearth Node on existing hardware

- Install the declared NixOS host/workload profile on an existing or commodity test machine.
- Run Home Assistant in a local test setup with simulated devices and test the documented offline path.
- Prove backup/export and recovery, least-privilege separation, update rollback, and that optional model/runtime failure does not block manual control.
- Capture real power, thermal, idle, performance, and acoustic measurements before making product claims.

**Exit evidence:** exact hardware/software profile, test corpus, run receipts, failures and claim ceiling. A synthetic device test is not physical electrical/safety certification.

### Gate C — Arc compatibility prototype

- Use existing x86-64 hardware before custom mechanical/electrical design.
- Freeze the compatibility-test format and test a diverse title set including native Linux, Proton, launcher, controller, suspend/resume, and known incompatibility cases.
- Record per-title performance/reliability and retest after material driver/runtime/hardware changes.

**Exit evidence:** versioned per-title compatibility matrix and repeatable benchmark method. No “any game” claim.

### Gate D — safe two-node Commons Compute prototype

- Keep the first workloads synthetic, public, short-lived, and independently verifiable.
- Demonstrate opt-in/off switch, owner limits, isolation, network denial, job cancellation, cleanup, resource accounting, and behavior during disconnect.
- Prove that malformed or dishonest results cannot be silently promoted to accepted output.

**Exit evidence:** fixed test manifests, adversarial corpus, exact artifact/profile digests, replay results and explicit unknown/failure states.

### Gate E — bounded robotics physical progression

- Follow existing Symthaea ROB-DESIGN / ROB-JOINTLAB dependencies.
- Use a fixed test article, measured calibration and declared mechanical/electrical envelope.
- Verify the physical stop, limits and abort behavior independently of Symthaea.
- Only then progress to a teleoperated base and a separately qualified low-risk task.

**Exit evidence:** exact as-built identity, calibration, test protocol, raw/derived observation separation, fault-injection results, physical-vs-simulation comparisons and limitations.

### Cross-product release rule

For every reported capability, publish the exact product/hardware/software/profile identity; source and artifact revisions; test environment and method; completed versus queued/incomplete versus failed checks; known limitations, privacy behavior, recovery path and service instructions; who validated the claim; and what it explicitly does not establish.

A queued test is not a pass. A schema-valid profile is not physical validation. A signed receipt is not proof of semantic correctness. A working device integration is not universal provider compatibility.

## 11. External standards and reference projects

Use these as implementation inputs rather than inventing equivalents:

- [Home Assistant: works without an internet connection](https://www.home-assistant.io/faq/works-without-internet)
- [Home Assistant: data privacy](https://www.home-assistant.io/faq/is-my-data-private/)
- [Steamworks: Steam Deck and Steam Machine compatibility review](https://partner.steamgames.com/doc/steamhardware/compat)
- [Steamworks: Proton and anti-cheat considerations](https://partner.steamgames.com/doc/steamdeck/proton)
- [Open Compute Project: Server specifications and modular systems](https://www.opencompute.org/projects/server/)
- [Open Compute Project: Modular Hardware System](https://www.opencompute.org/projects/server/mhs/)
- [Holochain: agent-centric, peer-to-peer application model](https://www.holochain.org/what-is-holochain/)
- [BOINC: computing with volunteers](https://github.com/BOINC/boinc/wiki/Computing-with-boinc) and [volunteer-computing security](https://github.com/BOINC/boinc/wiki/Security-in-volunteer-computing) — use as prior art and a threat-model baseline before deciding to build a new scheduler. BOINC documents result falsification, resource-credit falsification, malicious executable distribution, data-server abuse, and sandboxing concerns.
- [OSHWA definition of open-source hardware](https://oshwa.org/definition/) and [certification documentation checklist](https://certification.oshwa.org/process/documentation.html) — the preferred editable design source, BOM, explicit design scope, and suitable licensing matter; a generic PDF or a public repo alone is insufficient.
- [CERN Open Hardware Licence v2](https://cern-ohl.web.cern.ch/home) — provides permissive, weakly reciprocal, and strongly reciprocal variants. Choose per design after licensing review; do not imply one license fits every dependency.
- [ROS 2 security enclaves](https://design.ros2.org/articles/ros2_security_enclaves.html) and [ROS security keystore](https://docs.ros.org/en/rolling/Tutorials/Advanced/Security/Introducing-ros2-security.html) — use node/process-specific identity and permissions for robot communication rather than granting every process shared ambient trust.

## 12. Explicit nonclaims

This proposal does not claim a finished product, prototype, hardware design, tested bill of materials, or production supply chain exists; that a gaming system can legally or technically run every title; that community compute can execute arbitrary jobs safely without application-specific trust and validation; that Symthaea is conscious, independently authoritative, or ready for unsupervised physical control; that any robot/home device is safety-certified; or that a hardware/software profile is production-ready, externally audited, or qualified merely because this architecture is reviewed.

The first win is a small local home node that is useful, repairable, privacy-preserving, recoverable, and honest about what works. Gaming, compute-sharing, and robotics should grow from that foundation through independently reviewable gates.


## 13. Pre-hardware qualification plan

The implementation checklist, NixOS/QEMU VM lab topology, security and recovery test matrix, evidence-manifest requirements, physical-test limitations, and explicit go/no-go gate are maintained in [Pre-Hardware Qualification Plan](home-systems-prehardware-qualification-plan.md).

Treat this as a staged gate, not a promise of readiness: exact-head CI and required VM tests must pass before the team shortlists physical Hearth Node hardware. VM/simulation results cannot qualify thermals, electrical safety, physical emergency stops, radio behavior, or real GPU compatibility. The profile validator must remain fail-closed until a separately trusted verifier is reviewed and integrated.


## 14. Paradigm-shift research direction

The strategic direction is a **Sovereign Home Operating Environment**, not another all-in-one smart-home hub. The differentiators are owner-verifiable permission boundaries, essential-service continuity during failure, tested exit/migration, component-level serviceability, and evidence whose limitations are machine-readable.

The proposed paradigm shifts, research references, capability-contract design, Home Continuity Drill, and staged implementation programme are documented in [Paradigm Shifts for Open Home Systems](home-systems-paradigm-shifts.md). These remain hypotheses until the project measures them against commodity baselines and publishes repeatable results.

The first flagship acceptance demonstration should be a Home Continuity Drill in a disposable VM: WAN/DNS failure, optional-cognition failure, bounded resource pressure, failed update/recovery, and clean-environment restore. The report must retain per-invariant outcomes; a single aggregate score must never hide a critical failure.


## 15. Workload capability contracts

A first Draft 2020-12 declaration schema is published at [home-capability-contract.schema.json](../schemas/home-capability-contract.schema.json), with a design-only Hearth local-status example and negative fixtures for an empty egress allowlist and a critical physical effect missing safeguards.

This manifest is a **request and declaration**, not an authorization. It does not prove that the workload behaves as declared, that the host enforces its requested boundaries, or that any evidence is authentic. The owner-controlled policy and host enforcement layer must be separate from the workload-authored contract. Admission tests must demonstrate that an unapproved permission or network path is actually denied.

Capability contracts expose the intended paradigm shift: the owner approves a bounded request, the platform enforces it, and independent observation/reconciliation determines what happened.
