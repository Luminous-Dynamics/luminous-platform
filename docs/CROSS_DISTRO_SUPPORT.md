# Cross-Distribution Support Strategy

**Status:** Proposal / design baseline. This document does not claim that any non-NixOS distribution is currently supported.

## Decision

Build one Luminous Platform with explicit operating-system integration adapters. Keep NixOS the reference implementation, and add non-NixOS support by capability and component rather than by a single, ambiguous "Linux supported" label.

The first non-NixOS target should be **Ubuntu Server 24.04 LTS in an isolated QEMU test lane**. Advance to Ubuntu 26.04 LTS and Ubuntu Desktop only after the first lane passes. Fedora is the next target; assess Fedora bootc as a separate image-based deployment profile. Debian Stable follows once the adapter and test contract are proven.

This order optimizes for a reproducible, general-purpose Linux test target first, then adds an image-based fleet target. It is a prioritization proposal, not a claim that one distribution is intrinsically more secure than another.

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
| Other operating systems | Explicitly out of scope until a named profile, owner, and test lane exist. | Generic Linux claims, or implied macOS/Windows support. |

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
- [Ubuntu autoinstall reference](https://canonical-subiquity.readthedocs-hosted.com/en/latest/reference/autoinstall-reference.html)

The next implementation PR after this design should be the smallest evidence-backed extraction: first inventory the current Sovereign Boot boundaries and add a non-NixOS QEMU build/test lane. Do not start by generalizing all Nixward mutation paths.
