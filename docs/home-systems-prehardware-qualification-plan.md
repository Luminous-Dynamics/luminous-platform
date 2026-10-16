# Pre-Hardware Qualification Plan — Open Home Systems

**Status:** Engineering plan only; not evidence of implementation or qualification.  
**Scope:** Shared Luminous Platform foundations, with Hearth Node first, Arc Console and Commons Compute later, and physical robotics last.

**Decision rule:** Do not buy custom hardware or claim a product capability merely because a design, schema, VM test, simulator, CI job, receipt, or signed statement exists. The target is a system an ordinary owner can understand, operate locally, recover without the original developer, inspect, repair, and leave without losing their data or agency.

## 1. Define “best” in measurable terms

“Better than everything in the industry” is not testable until the comparison set, user needs, and metrics are defined. Aim to excel across the full lifecycle rather than assert universal superiority:

- **Owner sovereignty:** core features work without a vendor cloud/account; local control, export, deletion, and disablement remain available.
- **Resilience:** household essentials survive WAN outage, optional AI failure, service crashes, failed updates, and documented recovery events.
- **Security/privacy:** least privilege, narrow authority, understandable consent, secure defaults, maintained vulnerability response, and no secrets in prompts or public evidence.
- **Repairability/openness:** exact parts and limitations are documented; standard interfaces are preferred; replacement and service are realistic; openness claims identify proprietary firmware/blobs.
- **Interoperability:** build on Home Assistant, standard network/storage protocols, Steam/Proton, ROS 2, and mature ecosystems where they fit; do not fork without measured reason.
- **Usability:** accessible onboarding, reliable diagnostics, low-friction backups, understandable failures, and recovery by a non-developer.
- **Measured efficiency:** idle and workload power, energy per useful task, noise, thermal behavior, performance, lifecycle cost, and repair time are measured on exact hardware profiles.
- **Evidence integrity:** every claim names its exact subject, test, environment, limitations, and evidence class. Unknown remains unknown; stale evidence does not silently remain current.

Publish comparisons only when the method, versions, baseline, sample size, uncertainty, and results are available. “Best ever” is an aspiration; a reproducible, bounded claim is an engineering result.

## 2. Evidence classes and claim ceiling

Never collapse these classes into one “passed” badge:

1. **Designed:** written specification, contract, or architecture.
2. **Static-checked:** schema, linter, source analysis, configuration evaluation, dependency/license checks.
3. **Unit/contract-tested:** deterministic software tests.
4. **VM/simulated:** behavior in declared virtual machines, emulated devices, synthetic data, or simulators.
5. **Bench-measured:** instrumented results on exact physical hardware under stated conditions.
6. **Field-observed:** real operation over a stated duration and population, including denominators and failures.
7. **Independently reviewed/qualified:** a named method and assessor review the evidence and scope. This is not implied by a signature alone.

VM verification is valuable but cannot establish physical thermals, electrical safety, real RF behavior, a true hardware emergency stop, or actual hardware performance. Simulation, static checks, provider claims, and signed receipts must not substitute for physical evidence when physical behavior is the claim.

A signature can authenticate who produced a record and whether it changed. It does not prove that an experiment occurred, the equipment was genuine, the method was sound, or the conclusion follows. A future verifier must check provenance and content and apply a reviewed qualification policy.

## 3. Gate sequence

### Gate 0 — Product intent and threat model

- Write measurable user stories and acceptance criteria for the first product: **Hearth Node**, not all four products at once.
- Define supported/unsupported homes, networks, devices, service levels, accessibility needs, and operator assumptions.
- Draw trust boundaries among owner, host OS, Home Assistant, plugins, VM guests, downloaded models, Symthaea, Mycelix, update service, remote administration, and third-party devices.
- Model abuse cases: compromised plugin; malicious workload; stolen admin account; poisoned update; DNS compromise; cloud outage; lost encryption key; malicious household member/guest; supply-chain compromise; physical tampering.
- Assess consequence by device class. Lights/media are not equivalent to locks, heating, alarms, garage doors, medical equipment, or robots.
- Define recovery-time and recovery-point objectives by data/service class. Specify how long each service can safely be unavailable.
- Define competitor baselines and measurement protocols before collecting results.
- Maintain a risk register: severity, likelihood, owner, mitigation, residual risk, and authorized risk-acceptance decision.

**Exit:** reviewed requirements, data-flow/trust-boundary diagrams, threat model, service catalogue, measurable targets, and explicit nonclaims. This gate alone does not authorize hardware purchases.

### Gate 1 — Profile contract and evidence pipeline

- Make the CLI and Rust tests use one shared production semantic validator; avoid test-only duplicate logic that can drift.
- Validate the schema against the official Draft 2020-12 meta-schema and enable intended format validation.
- Add regression fixtures for malformed JSON, extra/unknown fields, missing required fields, wrong enums, missing safety booleans, stale/mismatched subject digests, fake receipts, receipts bound to another profile, non-passing results, duplicate records, invalid timestamps, qualified capability in a design-only profile, and each safety invariant.
- Test empty/unreadable fixture directories, invalid schema, filesystem errors, malformed input, and unexpected files. Contract failure must yield a non-zero exit code.
- Add property/fuzz tests for parser/validator crash paths and mutation tests proving negative controls detect weakened rules.
- For critical negative fixtures, assert the intended semantic rejection reason; do not pass only because the schema happened to reject the document.
- Define evidence manifests with subject digest, profile/version, tool and policy versions, input references, test IDs, result, environment, limitations, artifact/log digests, signer identity, and verifier result.
- Reject unknown, unsigned, revoked, expired, malformed, or out-of-policy evidence. Missing verifier, missing key, network error, unknown trust root, or unsupported evidence class must never auto-promote.
- Separate **receipt authenticity** from **qualification policy**. The verifier must retrieve or independently hash evidence and establish that required tests exist and match this profile.
- Document trust-root updates, key rotation/revocation, verifier independence, auditability, and privacy-safe logs.
- Preserve current negative controls; add controls rather than weaken them to make CI green.

**Exit:** one shared validator is covered by unit/integration tests; adversarial fixtures prove fail-closed behavior; exact-head CI succeeds. Queued is not passed.

### Gate 2 — Reproducible builds and supply chain

- Pin Nix inputs with committed lock data; avoid mutable URLs and unpinned downloads in release paths.
- Build from clean checkout using documented commands.
- Record source revision, flake-lock digest, architecture, Nix/toolchain versions, build inputs, and artifact hashes.
- Test offline/restricted-network rebuild after dependencies are cached; distinguish this from a fully hermetic build.
- Generate an SBOM; identify licenses, source availability, binary blobs, and redistribution constraints.
- Scan dependencies/images for known vulnerabilities; document exceptions, expiry dates, and owners.
- Prevent secrets from entering Nix store paths, derivations, logs, artifacts, or crash dumps.
- Define artifact signing/verification, release-channel provenance, update metadata authenticity, rollback policy, and compromised-signing-key response.
- Keep CI least-privileged and read-only unless a job has explicit need for writes.
- Pin third-party GitHub Actions to immutable commits and review pin changes deliberately.
- Verify artifacts correspond to exact source and locked inputs. A build badge or image name is not provenance.

**Exit:** a clean build reproduces declared artifacts to the documented standard; SBOM/license/security outputs exist; no untriaged release-blocking finding remains.

### Gate 3 — Host OS, service model, and management boundary

- Build a minimal declarative NixOS host profile for Hearth Node.
- Keep boot/recovery independent of Symthaea, Mycelix availability, DNS/Internet, UI rendering, community compute, and optional inference.
- Define service dependencies/start ordering; prove an optional service failure cannot break core boot.
- Give services dedicated unprivileged identities, read-only filesystems, minimal writable paths, reduced capabilities, syscall filtering, and only necessary device access.
- Apply systemd sandboxing where compatible and verify effective restrictions rather than assuming every control works in every runtime.
- Separate security domains for automation, storage, remote access, media, local inference, and untrusted compute.
- Never expose Docker/Podman sockets, Nix daemon sockets, hypervisor sockets, host PID namespace, or broad host paths to ordinary workloads.
- Keep admin authority out of web UIs and model tools by default. Separate proposal, authorization, dispatch, and postcondition verification.
- Provide local console/recovery access if UI or network is down.
- Test disk-full/inode exhaustion, read-only filesystem, failed mounts, missing services, crash loops, and corrupt configuration.
- Ensure critical alerts/controls do not rely only on cloud push or an optional dashboard.

**Exit:** minimal image boots in a VM; essential services start; optional dependencies can fail without breaking core local functions; privileged boundaries have tests.

### Gate 4 — VM lab and CI

Use NixOS tests/QEMU as the first integrated bench. Candidate disposable VMs:

- **hearth:** candidate NixOS host/profile;
- **automation:** Home Assistant with synthetic/emulated devices;
- **client:** household client and admin workstation;
- **untrusted:** hostile plugin or compute workload;
- **gateway:** controlled router/DNS/firewall for segmentation and outage injection;
- **backup:** isolated backup destination and restore target;
- **peer-a / peer-b:** later two-node Commons Compute lab.

Not every test needs every VM. Use explicit VLANs/networks to validate policy and verify actual attempted packets/connections are blocked. Never give guests access to host home directories, production credentials, SSH-agent sockets, the real home LAN, or privileged virtualization APIs.

VM scenarios:

- Clean install, first boot, reboot, simulated power loss, and service readiness.
- Offline boot and extended WAN/DNS outage; verify local automation, local UI, local services and data against the dependency contract.
- Network partition, latency, packet loss, DNS failure/poison, wrong clock, reconnect, and service recovery.
- Untrusted attempts to read host paths, reach LAN/metadata services, contact hypervisor, use secrets, escalate privilege, or monopolize resources.
- CPU/RAM/swap/disk/inode/PID exhaustion, oversized input/output, queue buildup, crash loops, and noisy-neighbour interference.
- Interrupted/corrupt updates, wrong signature, expired/revoked key, downgrade attempt, failed boot, rollback, and recovery.
- Missing/corrupt/full disk, damaged database, incomplete backup, offline backup target, restore to fresh VM, and integrity checks.
- Credential rotation, owner removal, remote-access disablement, export/delete behavior, and device trust revocation.
- Repeated restarts and long-running soak with memory/handle growth and service leaks recorded.
- Deterministic replay and failure injection; save test/source IDs and machine-readable outcomes.

Use fast unit tests and slower integration/soak tests. Each report needs test ID, outcome/reason, source revision, lock digest, VM definition, test-data digest, time, logs/artifacts. Missing, timed-out, skipped, or uncollected required tests are **incomplete**, not pass.

**Exit:** required VM matrix passes on the exact candidate revision; negative mutations demonstrate tests catch removed protections; reports are archived and replayable.

### Gate 5 — Identity, access, secrets, and remote management

- Define distinct permissions for owner, household member, guest, child/dependent, administrator, service, and recovery identities.
- Require strong auth/MFA for remote-capable admin accounts; offer secure local recovery without vendor cloud.
- Remote management is disabled by default or requires explicit documented enablement; no silent inbound listener or hidden relay.
- Use narrow, revocable credentials with rotation/expiry; never put secrets in prompts, logs, public evidence, crash reports, or peer-shared records.
- Test server-side authorization on the physical-effect path, not just UI visibility.
- Document account recovery, owner transfer, household separation, device removal, and secure disposal.
- Test compromised-session revocation and verify old tokens stop working.
- Make audit logs useful without collecting unnecessary household activity; define retention/deletion.
- Avoid shared default passwords, universal factory credentials, and undocumented support accounts.
- Establish security contact, vulnerability intake, response ownership, and disclosure policy before external users depend on the product.

**Exit:** automated positive/negative authorization tests cover the threat model; remote access and credential lifecycles are documented.

### Gate 6 — Privacy and data lifecycle

Inventory data source, owner, sensitivity, purpose, location, retention, sharing, backup, export, and deletion for: automation state/history, device names/presence, network/DNS logs, files/backups/keys, camera/microphone/transcripts/embeddings, prompts/context, robotics sensors, telemetry/crash dumps, diagnostics, evidence receipts, compute jobs, and peer metadata.

- Default to local processing and data minimization. No silent household-data transfer to remote AI/telemetry.
- Make camera/microphone activation visible/controllable; specify storage and deletion.
- Keep secrets/personal inputs out of public receipts and replicated peer storage.
- Provide machine-readable export and clear deletion, including backup/derived-copy limits stated honestly.
- Encrypt data at rest/in transit where applicable; define key ownership, unlock, rotation, recovery, destruction.
- Test permissions between services and data stores with synthetic household data.
- Scan logs, metrics, prompts, command args, error traces, backups, and diagnostics for data leaks.
- Document privacy implications of remote support, remote models, and community compute.

**Exit:** data-flow inventory reviewed; tests show synthetic canary secrets do not leave authorized boundaries.

### Gate 7 — Local automation and physical-effect authority

- Treat Home Assistant as an integration layer, not blanket trust for every add-on, integration, or device.
- Maintain a capability catalogue: allowed action, requester, risk class, prerequisites, bounds, and independent outcome observation.
- Preserve **observe → propose → authorize → dispatch → verify/reconcile**. API acknowledgement or sent command is not proof of physical effect.
- Model output cannot directly write privileged APIs, host files, secrets, actuators, or unrestricted shell.
- Unknown state, stale authority, missing observation, or timeout means **Unknown / Needs Reconciliation**, not assumed success.
- Classify action consequence. Locks, gates, heaters, alarms and safety-relevant devices need stronger gates and independent manual control than lights/media.
- Add rate limits, idempotency where supported, replay prevention, bounded retries, and safe cancellation.
- Test conflicting commands, races, duplicate delivery, stale telemetry, clock drift, broker restart, missing acknowledgements.
- Document manual override/local controls during host, UI, or network failure.

**Exit:** simulated effect tests show authorization, bounded dispatch, independent outcome observation, and honest unknown-state handling. This does not qualify a physical installation.

### Gate 8 — Updates, rollback, recovery, and data durability

- Define atomicity boundaries for system generation, service state, user data, and firmware; NixOS rollback does not automatically roll back mutable application data.
- Maintain distinct procedures for system rollback, application restore, host replacement, lost boot media, lost keys, and disaster recovery.
- Use encrypted backups and a failure-isolated destination; a second copy on the same disk is not a backup.
- Restore to a clean VM with new virtual disks and identity, then verify data/services.
- Test corrupted/malicious backup detection and recovery without booting a compromised system.
- Set and measure recovery-time/recovery-point objectives.
- Test first boot without Internet, update-fetch failure, interrupted transactions, exhausted disk, and rollback after bad release.
- Do not silently restore revoked credentials/vulnerable software through rollback.
- Provide offline recovery not dependent on production UI.
- Version data migrations and test compatibility/rollback policy.

**Exit:** clean-room restore and rollback scenarios pass with integrity checks and timing; non-developer recovery guide exists.

### Gate 9 — Resource and efficiency hypotheses

Before physical measurements, specify conditions and methods for idle/average/peak power, energy per task, boot-to-ready, idle CPU/RAM, storage IOPS/latency, network throughput, thermal throttling, acoustics, performance, automation latency, and recovery time. Define quotas/priorities for CPU/GPU/RAM/VRAM/storage/network, plus which workloads are shed first.

In VMs, verify quotas, priorities, bounded queues, and service availability under CPU/RAM/disk contention. Never label VM timing as physical performance or infer power, heat, sound, GPU performance, RF behavior, or storage endurance from ordinary VM results.

**Exit:** proposed budgets and reproducible physical measurement method exist; actual efficiency claims remain unqualified.

### Gate 10 — Usability, accessibility, and maintainability

- Test onboarding, device setup/removal, network changes, backup/restore, updates, permissions, and fresh-install recovery.
- Distinguish configured, running, healthy, reachable, observed, verified, degraded, and unknown; these are not synonyms.
- Provide useful diagnostics with timestamps, redaction, and corrective steps.
- Ensure export/recovery does not require the author, proprietary cloud, or paid subscription.
- Provide keyboard navigation, screen-reader semantics, scalable text, contrast, controller-friendly Arc UI, and localization strategy.
- Have a non-author execute critical tasks; record success, time, confusion, and recovery.
- Define support boundaries, deprecation, long-term update responsibility, end-of-life export, and repair path.
- Add documentation tests, release notes, known limitations, and migration guides.

**Exit:** critical owner tasks have repeatable procedures; usability defects have severity and owners.

## 4. Product-specific gates

### Hearth Node — first integrated target

Before selecting hardware, demonstrate in VMs:

- Declarative NixOS install/configuration and repeatable build.
- Local Home Assistant integration with synthetic/emulated devices.
- Local UI and essential automations during WAN/DNS outage.
- Service isolation and no ambient access to host admin interfaces/unrelated household data.
- Storage layout, backup, restore to clean VM, and integrity verification.
- Export/delete path and privacy-safe logs.
- Signed-update verification, failure injection, rollback, and manual recovery.
- Optional inference can be stopped, crashed, removed, or starved of resources without breaking boot, automation, storage, or operator controls.
- Offline owner operation and explicit remote-access configuration.
- Target envelope for power, noise, storage, thermal environment, repairability, component availability and total cost.

Then shortlist commodity candidates and record exact CPU/RAM/storage/NIC/radios, firmware/UEFI, TPM/Secure Boot behavior, PSU/cooling, replaceable parts, support window, driver status, and closed components. Compare BOM candidates by cost, idle power, acoustics, sustained thermals, performance, availability, repairability, and security. Do not freeze a BOM before limits and instruments are defined.

### Arc Console — defer separate hardware work

Build a host-independent compatibility harness first. Capture exact OS/kernel/driver/runtime/game versions, launch and interactive status, crashes, frame-time distribution, settings/resolution, controller/audio/display, suspend/resume, multiplayer/anti-cheat status, power/thermal capture, and evidence freshness. Keep per-title limitations and DRM/licensing constraints explicit. VMs can validate catalog, test-runner, provenance, and reports; they cannot establish real GPU/driver/gameplay compatibility.

### Commons Compute — two-VM adversarial proof first

Use two disposable nodes, synthetic public jobs, a controlled coordinator, and a hostile worker. Demonstrate explicit opt-in, owner limits, job isolation, egress denial, priority for local services, resource ceilings, cancellation, deletion, node removal, timeout, partial outputs, replay/duplicates, malicious result rejection, and independent output validation. Inject partition and coordinator loss. Do not distribute personal files, private model training, or household data in the first prototype.

### Symthaea Robotics — simulation/software-in-the-loop first

Define actuator/sensor contracts, units, limits, watchdog semantics, missing/stale observation behavior, command timeouts, and stop-state reporting. Record simulator/version and adversarial scenarios separately from physical evidence. A later physical joint lab requires independent electrical/mechanical review, a guarded area, current/force/speed limits, an independent physical emergency stop appropriate to the rig, safe power removal, and instrumentation. A software flag or VM test cannot qualify a physical stop.

## 5. Adversarial scenario catalogue

Automate or explicitly justify exclusion of each applicable scenario:

- WAN/DNS/time-source outage, clock skew, expired certificate.
- Router reboot, split network, packet loss, latency, duplicates, reconnect storm.
- Home Assistant crash/restart, integration disappearance, stale/malformed state, database corruption.
- Inference unavailable, malicious model output, prompt injection via device names/content, huge output, repeated requests.
- Compromised admin UI; ordinary user attempts admin action; revoked token reuse; stolen key; account removal.
- Malicious/vulnerable plugin; dependency compromise; unsigned/wrongly signed/stale update; revoked signer; downgrade.
- Malformed config, schema evolution, unknown keys, parser fuzzing, invalid receipt data.
- Disk/inode full, storage latency, corrupt disk image, missing mount, backup offline, restore under a fresh identity.
- CPU/memory/storage/network/process/log exhaustion and noisy-neighbour effects.
- Host/guest boundary attempts, host-socket discovery, shared-directory access, forbidden network paths, VM-isolation review.
- Interrupted update at transaction boundaries, failed activation, rollback, restart loops.
- Duplicate/reordered commands, timeout after dispatch, acknowledged command with unknown effect, contradictory observations, manual-override race.
- Export/deletion, log/artifact secret scan, retention expiry, backup deletion limits, account/device offboarding.
- Long soak and repeat reboots; leaks, queue build-up, stale metrics, unexplained state drift.
- Corrupt/incomplete evidence, missing logs, wrong subject digest, reused receipt, altered manifest, unknown/revoked key, stale evidence, skipped required test.

A test file existing is not proof that a scenario is covered. Assertions must detect the fault; controlled negative mutations should show that removing the protection causes test failure.

## 6. Required qualification evidence bundle

Each run needs a machine-readable manifest containing:

- Unique run ID, policy version, repo/commit/tree digest, lock digest, profile ID/version/digest.
- Exact test list, test runner/tools, VM definitions, architecture, virtual devices, network topology, and injected failures.
- Input/fixture digests without publishing private inputs.
- Start/end time, clock source/uncertainty, environment, logs/artifacts and hashes.
- Per-test outcome: pass, fail, incomplete, unknown, or not applicable with rationale.
- Expected/observed values, tolerances, deviations, resource use, issues.
- Signer/key ID, verification status, verifier version, independent reviewer decision.
- Evidence class (especially VM/simulated), limitations, and explicit claim ceiling.

Missing required test/log, unmatched subject, invalid signature, unknown trust root, or verifier error means no qualification. Public evidence must not contain credentials, personal sensor data, private household state, or unrestricted private job inputs.

## 7. Go/no-go checklist before hardware testing

- [ ] Requirements, supported scope/exclusions, threat model, trust diagram, and risk register reviewed.
- [ ] CI tests the shared production validator; no duplicated test-only semantic policy.
- [ ] Positive/negative fixtures cover required invariants; fuzz/property/mutation strategy defined; negative controls retained.
- [ ] Exact-head CI and required VM suite succeed; queued/cancelled/skipped/stale-head runs do not qualify.
- [ ] NixOS profile builds from clean checkout with build limitations documented.
- [ ] Local essentials work without WAN/cloud/optional cognition/community compute within the declared scope.
- [ ] Untrusted workload isolation and egress denial are demonstrated by VM network tests.
- [ ] Privilege boundaries, owner consent, remote access, secrets, export/delete, and log redaction are tested.
- [ ] Update authenticity, failure injection, rollback, clean-room restore, and manual recovery have evidence.
- [ ] Resource contention/service-priority behavior meet defined VM thresholds.
- [ ] Evidence bundle format and fail-closed verifier policy are specified; signatures are not called proof of physical truth.
- [ ] Hardware shortlist rubric, parts/support inventory, lifecycle cost model, and measurement plan are ready.
- [ ] Claims planned for the first prototype and required evidence for each are documented.
- [ ] A named owner can stop a test when a safety/security invariant is violated.

**Go/no-go principle:** if a requirement cannot be tested in a VM, document why, the required physical measurement, hazard controls, instrument, acceptance threshold, and responsible person. “Cannot test in a VM” means plan a physical test—not skip the requirement.

## 8. What VMs cannot qualify

Physical tests remain mandatory for actual input power/energy, acoustics, sustained thermals, GPU/game compatibility and performance, storage endurance, board/firmware/UEFI/TPM/Secure Boot behavior, real Wi-Fi/Zigbee/Thread/Bluetooth/RF coexistence, actual adapter/driver faults, electrical safety, physical emergency stops, robot dynamics, sensor calibration, and long-term environmental effects.

Use VMs to eliminate architectural ambiguity, validate software logic, rehearse recovery, and reduce expensive hardware iterations. Do not use host-shared folders, device passthrough, or bridged networking by default; scope them explicitly and never expose developer secrets or the household LAN.

## 9. Implementation order

1. Refactor the profile validator so CLI and tests use one semantic implementation; fix the fabricated-receipt assertion to check the actual fail-closed error.
2. Complete the contract gate on the exact PR head and preserve all negative controls.
3. Run the first NixOS/QEMU sentinel test under tests/home-systems-vm/ to verify core-service survival when optional inference fails; replace sentinel units with the real Hearth modules before claiming product evidence.
4. Add hostile-workload multi-VM network-isolation tests.
5. Add clean-VM backup/restore plus update/rollback fault injection.
6. Publish qualification-manifest format and explicit claim-state semantics.
7. Only after shared gates pass, shortlist commodity Hearth hardware and instrument a bench protocol.
8. Keep Arc Console, Commons Compute, and robotics as separate later gates; Hearth passing does not qualify them.

## References

- NixOS Manual — VM builds and the runNixOSTest framework: https://nixos.org/manual/nixos/stable/
- QEMU System Emulation — security architecture and guest isolation: https://www.qemu.org/docs/master/system/security.html
- QEMU — network emulation: https://www.qemu.org/docs/master/system/devices/net.html
- Home Assistant — securing your installation: https://www.home-assistant.io/docs/configuration/securing/
- NIST IR 8259 Rev. 1 (April 2026), Foundational Cybersecurity Activities for IoT Product Manufacturers: https://csrc.nist.gov/pubs/ir/8259/r1/final
- NIST IR 8259A, IoT Device Cybersecurity Capability Core Baseline: https://csrc.nist.gov/pubs/ir/8259/a/final
- systemd execution environment and sandboxing options: https://github.com/systemd/systemd/blob/main/man/systemd.exec.xml

These sources inform the plan; they do not qualify a Luminous implementation.


## 10. Initial VM scaffold added

The repository now includes `tests/home-systems-vm/hearth-foundation.nix`, a NixOS test module, plus `run.nix` and a README with run instructions. The first test deliberately fails an optional inference sentinel and asserts that the core-service sentinel remains active/restartable and does not require inference. The profile validator also rejects symlinks during fixture traversal (avoiding directory-cycle and external-path surprises), emits rejection reasons, and now has focused regression tests for design-only, robotics-stop, qualified-capability and symlink invariants.

This is executable test scaffolding, but it is not yet run evidence. It uses test-double systemd units because a product Hearth NixOS module does not yet exist in this PR. The preferred runner is now the isolated `tests/home-systems-vm/flake.nix` with committed `flake.lock`; the GitHub Actions workflow runs it on matching changes and uploads a run manifest plus logs. The legacy `run.nix` path still uses caller-supplied `<nixpkgs>` and is not the pinned qualification path. This remains test-double evidence until the VM run succeeds and the sentinel units are replaced with real Hearth modules.


## 11. Paradigm-shift acceptance criteria

The project direction is captured in [Paradigm Shifts for Open Home Systems](home-systems-paradigm-shifts.md). Treat the following as testable hypotheses, not promised product advantages:

- **Home Continuity Drill:** retain essential local capability while WAN/DNS and optional cognition are unavailable; test real dependency edges, not just process presence.
- **Capability contract admission:** every workload declares subject identity, permissions, data handling, network, resource, failure, authority, and evidence requirements. The manifest is a request/declaration, never a grant by itself; the owner policy and enforcement mechanism must independently restrict it.
- **Exit and migration:** export configuration/data and restore to a clean replacement VM without contacting a vendor-specific cloud account; document data/feature loss.
- **Evidence freshness:** a change to the claimed subject, relevant dependency, policy, verifier, or test harness causes affected claims to be re-evaluated rather than carried forward silently.
- **Lifecycle quality:** before hardware selection, define measurements for power, serviceability, supported parts, update lifetime, noise, thermals, and lifecycle cost; do not estimate physical performance from VM timings.

Do not create a combined “sovereignty score” that can average away a critical failure. Publish a per-invariant result vector with the exact test subject, observed outcomes, and claim ceiling.
