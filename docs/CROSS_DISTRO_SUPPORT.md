# Cross-Distribution Support Strategy

**Status:** Proposal / design baseline. This document does not claim that any non-NixOS distribution is currently supported.

## Decision

Build one Luminous Platform with explicit operating-system integration adapters. Keep NixOS the reference implementation, and add non-NixOS support by capability and component rather than by a single, ambiguous "Linux supported" label.

The first non-NixOS target should be **Ubuntu Server 24.04 LTS in an isolated QEMU test lane**. Advance to Ubuntu 26.04 LTS and Ubuntu Desktop only after the first lane passes. Fedora is the next target; assess Fedora bootc as a separate image-based deployment profile. Debian Stable follows once the adapter and test contract are proven.

This order optimizes for a reproducible, general-purpose Linux test target first, then adds an image-based fleet target. It is a prioritization proposal, not a claim that one distribution is intrinsically more secure than another.

## Host operating system versus guest operating systems

Do not conflate native host support with guest availability. **NixOS is the preferred reference host for machines we control; OCI containers and VMs are complementary workload mechanisms, not a substitute for native host qualification.** A tested Ubuntu guest running on NixOS does not prove that Luminous host services support Ubuntu natively.

The proposed runtime layering and workload-placement policy are documented in [Host and Workload Strategy](HOST_AND_WORKLOAD_STRATEGY.md). Initial recommendation: a declared NixOS host, rootless OCI containers for suitable services, and one microvm.nix NixOS guest proof. Use a general KVM/QEMU manager for heterogeneous OS guests. Do not introduce multiple VM control planes until the workload needs are proven.

## Synology DSM: a separate NAS appliance track

Synology support is worth pursuing, but **DSM is not simply another Ubuntu/Fedora target**. Treat it as a version- and model-qualified appliance integration, with storage availability and vendor-managed lifecycle taking precedence over host customization.

Recommended order has three layers. **First, add remote, read-only SNMPv3 monitoring** from an already-supported Luminous host, using Synology's documented MIBs with authentication and privacy enabled. This provides system, disk, RAID, and optional UPS health without installing code on DSM or requiring DSM API credentials. **Second, optionally add a narrow DSM Web API adapter** only for fields unavailable through documented MIBs; it must discover the API catalog and use documented, allowlisted operations rather than assuming private `SYNO.Core.*` APIs are stable. **Third, optionally run a headless Luminous service on the NAS** through Container Manager/Compose, but only on a model/version where Synology explicitly supports it. Do not install a native SPK package or change DSM boot behavior in the first milestone.

Container Manager availability is model- and DSM-version-dependent. Synology's current product matrix lists supported models, and its release notes show changing minimum DSM versions across releases; therefore, qualification must record the exact NAS model, CPU architecture, DSM build, and Container Manager version rather than saying "all Synology NAS". See [Container Manager applied models](https://www.synology.com/en-in/dsm/packages/ContainerManager) and [Container Manager release notes](https://www.synology.com/en-us/releaseNote/ContainerManager).

### Scope by component

- **Good initial candidates:** headless Rust services that work in a container, read-only health/metrics endpoints, local-only administration, and optional Xenia network components after their headless service mode is validated.
- **SNMPv3 monitor (first implementation):** use Synology's documented SNMP MIBs from an external collector. Require SNMPv3 with both authentication and privacy (authPriv), restrict UDP/161 at the NAS firewall to the collector, keep credentials in a secret store, and use a bounded OID allowlist with read-only operations. Synology explicitly recommends SNMPv3 for security and documents its system, disk, RAID, and UPS MIBs. See the [DSM SNMP guide](https://kb.synology.com/en-ca/DSM/help/DSM/AdminCenter/system_snmp?version=7) and [Synology MIB guide](https://kb.synology.com/en-af/DG/Synology_DiskStation_MIB_Guide/5).
- **DSM Web API adapter (second, optional):** run outside DSM and use it only for fields that cannot be obtained through documented MIBs. Discover paths and version ranges through `SYNO.API.Info`; do not hard-code CGI paths or assume private `SYNO.Core.*` APIs are stable. Use HTTPS, a dedicated least-privilege account, request timeouts, bounded retries, and redacted logs. Missing APIs or denied permissions are `unknown`, not healthy. See the [DSM Login Web API Guide](https://kb.synology.com/en-global/DG/DSM_Login_Web_API_Guide/2).
- **Not in the first milestone:** Sovereign Boot animation or recovery in DSM's boot path; Nixward mutation; bootloader/initramfs/kernel changes; DSM package replacement; privileged containers; broad host mounts; Docker socket access for a control-plane agent.
- **Later option:** a native DSM `.spk` package only if Container Manager cannot meet a defined need. DSM package development has its own framework, architecture mapping, permission model, and privilege constraints. Require a separate threat model and package-lifecycle tests before pursuing it. See Synology's [DSM 7.2.2 Package Developer Guide](https://global.download.synology.com/download/Document/Software/DeveloperGuide/Os/DSM/All/enu/DSM_Developer_Guide_7_enu.pdf).

A containerized deployment is not equivalent to native DSM integration. It gives us useful NAS-resident services while preserving the appliance boundary, but does not by itself demonstrate DSM API, storage, or failover compatibility.

The draft machine-readable starting contract is [`profiles/synology-dsm-snmpv3.yaml`](../profiles/synology-dsm-snmpv3.yaml). It is design-only—not a validated collector configuration—and requires exact model/DSM/MIB qualification before implementation claims. The profile deliberately omits serial numbers and SMART raw attributes by default.

The implementation rationale, Rust client-library evaluation, strict parsing policy, and first CI test gates are recorded in [ADR: Synology DSM monitoring starts with read-only SNMPv3](SYNOLOGY_MONITOR_ADR.md).

## Product boundaries

Do not turn every project into a cross-distro configuration manager.

| Component | Cross-distro direction | Keep distribution-specific |
|---|---|---|
| `sovereign-boot` | Make the Rust renderer, lifecycle receipts, and optional Linux integration independently buildable and testable. | NixOS module, activation defaults, and any recovery operation that depends on NixOS generations. |
| `nixward` | Preserve reusable observation, evidence, policy, and transaction concepts where they can be separated without semantic loss. | Nix evaluation, flakes, store/generation operations, and NixOS-specific mutation remain NixOS-only until a separately designed backend proves otherwise. |
| `luminous-edge` / Luminous Platform | Describe component capabilities, installation profiles, minimum requirements, test evidence, and tested version pairs. | Keep Nix flake composition as the NixOS delivery path; add other delivery paths only when tested. |
| `xenia-peer` | Keep the remote-session application independent of NixOS as far as its dependencies allow; test permissions and desktop integration per environment. | Screen capture, input injection, portals, codecs, and service/session setup remain capability- and desktop-specific. |
| `xenia-wire` | Remain transport- and OS-agnostic at the protocol layer. | OS-specific transports or key storage belong in adapters above the wire format. |

## Integration architecture

Keep policy and evidence semantics above the distribution adapters. A future shared host-integration boundary should expose narrow, typed operations rather than a generic privileged shell:

1. **Observe:** collect OS identity/version, init system, available capabilities, service state, graphics devices, boot mode, and relevant package/boot-management mechanisms. Mark unavailable and unknown values explicitly.
2. **Plan:** produce a deterministic description of proposed effects and prerequisites. Planning must not mutate the host.
3. **Validate:** verify the exact host pre-state, artifacts, target versions, ownership/permissions, and recovery prerequisites. A changed pre-state invalidates the plan.
4. **Authorize:** require an explicit operator authorization bound to the canonical plan digest and host identity. Cognitive scores, natural-language intent, or a generic "yes" are not authorization.
5. **Apply:** call a small, allowlisted backend operation. No interpolated shell command or ambient authority.
6. **Verify:** observe the resulting state independently and emit a receipt bound to the plan, actual effects, and verification results.
7. **Recover:** use a backend-specific, documented recovery mechanism. Unknown or indeterminate outcomes must remain indeterminate; do not claim rollback succeeded without evidence.

This is an architectural target, not an existing cross-distro API. Do not extract or generalize code until an inventory identifies real shared semantics and their tests.

## Support matrix (planned gates)

| Target/profile | Initial scope | Must not imply |
|---|---|---|
| NixOS | Existing reference path: flake/module integration and Nixward-specific management. Re-run relevant existing gates when shared code changes. | That every component is production-ready merely because the NixOS path exists. |
| Ubuntu Server 24.04 LTS, QEMU | First non-NixOS proof: Rust build, package/service integration, read-only observation, disabled-by-default boot animation, failure isolation, and evidence capture. | Physical-hardware qualification, desktop input integration, or automatic OS mutation. |
| Ubuntu Server 26.04 LTS | Follow-on compatibility lane after 24.04 passes. Pin the image and package versions used by the evidence. | Support for every Ubuntu derivative or release. |
| Ubuntu Desktop | Separate graphics/session qualification: DRM/KMS contention, display-manager handoff, Wayland behavior, suspend/resume, and user-session permissions. | That a server VM passing is sufficient for desktop use. |
| Fedora Workstation/Server | Add an RPM/service integration profile and independent VM evidence. | That RPM-family support automatically qualifies Fedora bootc. |
| Fedora bootc | Separate image-based profile: immutable/managed filesystem assumptions, booted deployment identity, deployment promotion, and rollback verification. | That package installation or mutation works like a traditional mutable host. |
| Debian Stable | Add after the common contract stabilizes; test independently even when an Ubuntu adapter can be reused. | That Ubuntu results establish Debian support. |
| Synology DSM (exact model/build) | First: remote read-only SNMPv3 monitor using documented Synology MIBs. Optional second step: narrow DSM Web API adapter. Optional third step: supported Container Manager + Compose, unprivileged containers, and dedicated persistent directory. | Generic Linux support, native boot/recovery integration, Nixward management, or support for all NAS models/architectures. |
| Other operating systems | Explicitly out of scope until a named profile, owner, and test lane exist. | Generic Linux claims, or implied macOS/Windows support. |

### Keep host and guest support separate

Publish two support matrices. The **host matrix** records which native OS release/hardware profiles qualify each Luminous component. The **guest matrix** records OCI image architecture/digest and VM guest OS/image/version plus the tested runtime/VMM. OCI image portability, a VM starting, a guest OS booting, and full Luminous native integration are separate capability claims.

### Report support by capability

Do not use one boolean for distribution support. Publish a per-profile matrix for:

- builds and package provenance;
- read-only host observation;
- systemd service lifecycle;
- DRM/KMS boot visuals;
- desktop/session integration;
- system configuration changes;
- boot selection and recovery;
- unattended upgrades and rollback;
- Xenia capture and input permissions;
- tested architectures, virtual hardware, and physical hardware.

Each cell should be one of **not implemented**, **implemented but unverified**, **verified in CI**, or **qualified on named hardware**, with a link to the exact evidence. A queued workflow is not a pass.

## Ubuntu-first implementation plan

### Gate 0 — inventory and design

- Enumerate each current `sovereign-boot` binary, Nix module, recovery action, privileged file path, service unit, environment assumption, and boot/display interaction.
- Classify every operation as portable Rust logic, Linux-specific observation, NixOS-specific management, or unresolved.
- Freeze a versioned host-profile manifest and define what each receipt records.
- Keep recovery and bootloader mutations out of the first Ubuntu milestone.

### Gate 1 — disposable QEMU lane

- Pin an Ubuntu Server 24.04 LTS image by digest and record the image provenance.
- Use UEFI/OVMF and virtual graphics devices needed to exercise Linux DRM/KMS; document the exact QEMU machine configuration.
- Test install, uninstall, missing binary/configuration, invalid state file, interrupted service start, permission denial, and service failure.
- Prove the base OS still boots when the Luminous service is absent, disabled, misconfigured, or crashes.
- Keep the feature disabled by default and never enable it on the developer's physical host as a side effect of the test.
- Capture serial logs, exit status, test manifest, image digest, binary hashes, and verifier output. A test result is passing only when the assertions and independent checks pass.

### Gate 2 — packaging and service integration

- Choose a versioned `.deb` or signed, checksummed tarball only after the inventory shows which dependencies the first milestone actually needs.
- Keep package install/remove separate from enabling the service.
- Require an explicit action and preflight receipt before enabling boot visuals.
- Do not modify GRUB, EFI boot entries, initramfs, kernel arguments, or the default boot path in the first milestone.
- Test systemd hardening, user/group ownership, state-directory permissions, restart behavior, logs, and fail-open behavior.
- Provide an uninstall path that removes only files created by the package and preserves user state unless the operator explicitly chooses otherwise.

### Gate 3 — desktop qualification

- Test on Ubuntu Desktop in QEMU before physical hardware.
- Validate clean handoff between DRM/KMS rendering and the display manager; a renderer must release graphics resources and must not leave the user without a normal login if it fails.
- Test Wayland session portals and permissions independently for Xenia. Do not treat a working boot animation as evidence for screen capture or input injection.

### Gate 4 — Fedora and bootc

- Add conventional Fedora using a distinct RPM/service profile.
- Evaluate Fedora bootc as a separate immutable, image-based profile with explicit deployment identity and rollback semantics.
- Require proof of the actual booted deployment and resulting health before any promotion claim.

### Separate Gate S0 — read-only Synology SNMPv3 monitor

- Run the collector on an already-supported Luminous host; install nothing on the NAS.
- Require SNMPv3 authPriv (authentication plus privacy); explicitly reject SNMPv1, SNMPv2c, noAuthNoPriv, and authNoPriv in the supported configuration.
- Restrict UDP/161 at the NAS firewall to the collector source address and prefer a management VLAN/VPN; do not expose SNMP to the public internet.
- Start with a fixed allowlist: standard SNMP system identity/uptime, Synology system status/temperature/fan/CPU/memory utilization, disk status/health/temperature, RAID/storage status and capacity. Do not collect serial numbers or SMART raw attributes by default.
- Use a poll interval of at least 60 seconds, bounded retries/timeouts, a row cap for table walks, and strict limits on OID subtree traversal.
- Map numeric values using the Synology MIB definitions for the qualified DSM family; do not treat unknown enum values or missing OIDs as healthy.
- Test timeout, bad credentials, auth/privacy failure, unsupported OID, malformed/truncated responses, table index changes, and partial observations.
- Record target address/identity alias, poll timestamp, source MIB/profile version, OIDs read, raw typed values (excluding secrets), and normalized status in a receipt. Never record SNMP auth/privacy passwords.
- Keep serial numbers, SMART raw attributes, installed-package inventory, and network/user details out of the initial profile unless a specific need and data-minimization review justify them.

### Separate Gate S1 — optional DSM Web API adapter

- Run outside DSM; use only for fields unavailable through the documented MIB allowlist.
- Discover the API catalog and version/path metadata first through `SYNO.API.Info`; do not hard-code `entry.cgi` versus `auth.cgi` or assume private `SYNO.Core.*` APIs are portable.
- Require HTTPS with certificate validation; do not quietly accept a self-signed certificate without an explicit, pinned trust configuration.
- Use a dedicated least-privilege account and a narrow documented read-only API allowlist. Sessions and tokens are secrets; scrub them from logs and evidence.
- Missing APIs, permission denial, unsupported schema, expired sessions, and partial responses become unknown/error, not healthy.
- Do not introduce Web API login credentials until SNMPv3 cannot satisfy a defined requirement.

### Separate Gate S2 — optional Synology NAS container profile

- Select one real target NAS only after confirming exact model, CPU architecture, DSM version/build, storage configuration, and Container Manager availability.
- Build a multi-architecture image only for the architecture actually needed; pin image digests and software versions. Do not assume an x86_64 build will run on ARM models.
- Supply a minimal Compose project: non-root UID/GID, explicit resource limits, restart policy, no privileged mode, no host PID/network namespaces, no Docker socket, and no device access.
- Mount only one dedicated application data directory; make the application binary and configuration read-only where practical. Never mount NAS data shares wholesale.
- Start with local-only health/status endpoints and outbound connections. Expose ports only after documenting the network threat model and firewall requirements.
- If DSM APIs are needed, use HTTPS with certificate validation, a dedicated least-privilege account, minimum required documented read-only API operations, token redaction, and an API catalog discovered from the NAS. Treat API absence/permission errors as unknown, not as healthy.
- Test container restart, crash loops, full disk, permission denial, unavailable share, NAS reboot, DSM update/restart, image rollback, backup/restore, and uninstall without data loss in a disposable/test environment.
- Keep container management separate from the agent's authority: never give the Luminous control plane access to the Docker socket or an unrestricted SSH/root credential.
- Publish qualification only for the tested model, DSM build, architecture, Container Manager version, and test evidence.

Do not pursue an SPK installer, root-privileged package, or automatic DSM modification until a concrete requirement cannot be met by this container profile and the DSM package threat model has been reviewed.

## CI and qualification contract

Every distro lane should record:

- distribution, exact release, architecture, image digest, and test harness version;
- repository commit, artifact hashes, package metadata, and dependency lock state;
- tests run and exact exit status;
- pre-state and post-state observations;
- boot continuity and recovery assertions;
- verifier version and receipt digest.

Use QEMU for deterministic automated coverage, then hardware qualification for DRM/KMS, GPU drivers, suspend/resume, and desktop/session-specific behavior. CI coverage and hardware qualification are different claims.

Fail closed for authorization and evidence validation, but fail open for optional presentation services: if the renderer, observer, or cognitive layer is unavailable, normal OS boot and login must continue. Never silently downgrade failed verification to success.

## Explicit non-goals for the first release

- Replacing NixOS or making Nixward a generic imperative package manager.
- Cross-distro package installation, kernel/bootloader mutation, or autonomous OS upgrades.
- Enabling boot visuals automatically during installation.
- Claiming universal Linux, desktop-environment, GPU, or physical-hardware support from a VM test.
- Requiring Xenia, Symthaea, or any platform daemon for host boot.
- Treating an AI recommendation, confidence score, or natural-language instruction as authority to mutate a machine.

## Upstream design references

- [Luminous Platform](https://github.com/Luminous-Dynamics/luminous-platform)
- [Sovereign Boot](https://github.com/Luminous-Dynamics/sovereign-boot)
- [Nixward](https://github.com/Luminous-Dynamics/nixward)
- [bootc project and documentation](https://github.com/bootc-dev/bootc)
- [Host and Workload Strategy](HOST_AND_WORKLOAD_STRATEGY.md)
- [NixOS Manual: VM versus container trade-offs](https://nixos.org/manual/nixos/stable/)
- [microvm.nix](https://github.com/microvm-nix/microvm.nix)
- [Podman Quadlet](https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html)
- [Incus](https://github.com/lxc/incus)
- [Firecracker design](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md)
- [Ubuntu autoinstall reference](https://canonical-subiquity.readthedocs-hosted.com/en/latest/reference/autoinstall-reference.html)
- [Synology Container Manager supported models](https://www.synology.com/en-in/dsm/packages/ContainerManager)
- [Synology Container Manager release notes](https://www.synology.com/en-us/releaseNote/ContainerManager)
- [Synology DSM SNMP settings and security](https://kb.synology.com/en-ca/DSM/help/DSM/AdminCenter/system_snmp?version=7)
- [Synology SNMP MIB Guide](https://kb.synology.com/en-af/DG/Synology_DiskStation_MIB_Guide/5)
- [DSM Login Web API Guide](https://kb.synology.com/en-global/DG/DSM_Login_Web_API_Guide/2)
- [Synology DSM Developer Guide](https://global.download.synology.com/download/Document/Software/DeveloperGuide/Os/DSM/All/enu/DSM_Developer_Guide_7_enu.pdf)

The next implementation PR after this design should be the smallest evidence-backed extraction: first inventory the current Sovereign Boot boundaries and add a non-NixOS QEMU build/test lane. Do not start by generalizing all Nixward mutation paths.
