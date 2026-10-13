# Luminous Host and Workload Strategy

**Status:** Proposed architecture, not an implementation or performance claim.
**Decision question:** Should NixOS be the primary host, with VMs and containers providing fast, isolated environments for multiple operating systems?

## Decision

**Yes—with a boundary:** make NixOS the preferred, reproducible Luminous reference host for machines we control, while keeping Luminous Platform's host integrations and workloads portable. Do not require every computer or appliance to run NixOS, and do not confuse "a Linux OS runs as a guest" with "the platform natively supports that OS."

The target architecture has three independently qualified surfaces:

1. **Host integration:** sovereign-boot, observability, hardware/session integration, and Nixward where applicable. NixOS is the reference. Ubuntu, Fedora, Debian, and Synology DSM require their own documented profiles.
2. **Workload packaging:** portable OCI images for suitable services, preferably running rootless under Podman or a similarly bounded runtime. An OCI image does not require the host to be NixOS.
3. **Guest environments:** KVM-backed VMs for guest kernels, different operating systems, kernel testing, high-risk workloads, and stronger isolation; lighter microVMs for workloads whose requirements match their narrower device model.

NixOS should provide a declarative base, pinned build and configuration inputs, reproducible service definitions, rollback/recovery, and controlled virtualization. It is not a reason to replace customer systems, bypass a NAS vendor's boundaries, or move every workload into a VM.

## Recommended first stack

### Baseline: NixOS + OCI containers

Use NixOS for host configuration and use OCI images for services that do not need their own kernel. Begin with Podman and rootless containers where feasible; manage persistent services declaratively and avoid mutating containers as a substitute for versioned images. Pin image digests and resource/permission policy. Rootless alone does not establish strong isolation: still use least privilege, namespaces, seccomp/LSM policy when available, bounded mounts, and explicit network policy.

This is the simplest path to run many application services while preserving a deterministic NixOS host. It is also the most portable workload format across qualified Linux hosts.

### Second: microvm.nix for NixOS guests

Evaluate [microvm.nix](https://github.com/microvm-nix/microvm.nix) for NixOS workloads that benefit from a separate kernel and a small device model. It supports several VMM backends, including QEMU, Cloud Hypervisor, and Firecracker, with different device, network, and share limitations. This ecosystem fits NixOS guests and declarative flake-based deployment well. It does **not** automatically mean arbitrary Ubuntu, Debian, Windows, or other guest images are managed through a NixOS microVM definition.

Do not pick a VMM solely because it is described as "fast." Benchmark cold boot, memory floor, CPU overhead, disk and network throughput, lifecycle reliability, observability, and recovery using actual target workloads.

### Third: full guest OSes for compatibility

To test or host Ubuntu, Debian, Fedora, and other guest OSes, use a general VM manager with KVM/QEMU, such as libvirt or Incus. Choose one first control plane rather than starting with libvirt, Incus, and microvm.nix all managing overlapping workloads.

- **libvirt/QEMU:** conservative choice for conventional full VMs and established VM lifecycle operations.
- **Incus:** attractive if unified system-container and VM lifecycle, image management, and a remote management API are core requirements. However, access to the Incus administrative socket is effectively host-root-equivalent; never grant it to a general-purpose cognitive agent or remote operator by default.
- **microvm.nix:** preferred initial experiment for lightweight, declarative **NixOS** guests, not a replacement for the general guest OS matrix.

Decision gate: start with Podman/OCI and a minimal microvm.nix proof of concept. Add libvirt or Incus only after a defined use case demands heterogeneous full-OS guest lifecycle management. Do not operate multiple hypervisor/control planes until their resource ownership, firewalling, storage, and failure boundaries are explicit.

### Optional later: Firecracker

Firecracker may be worth a later high-density Linux microVM experiment. It requires KVM and its production guidance calls for using the jailer. Its constrained device model makes it appropriate for isolated Linux services, not a universal replacement for desktop or general-purpose VMs. Do not make it the first platform-wide default.

## Workload placement policy

| Requirement | Default placement | Rationale and constraint |
|---|---|---|
| Small, trusted stateless service | OCI container | Low overhead, simple rollout; bound capabilities, network, resources, and mounts. |
| Stateful database or service | OCI container or host service with explicit data lifecycle | Backups, durability and restore semantics matter more than cold-start speed. Never hide state in an ephemeral writable layer. |
| NixOS-specific service/appliance | Declarative NixOS service or microvm.nix guest | Reuses the Nix build/config model while retaining an explicit rollback boundary. |
| Ubuntu/Debian/Fedora compatibility test | Full KVM VM with pinned image provenance | Tests the actual guest kernel, init system, packages, and system behavior. |
| Untrusted third-party code or stronger tenant boundary | VM/microVM, according to threat model | Separate guest kernel narrows exposure versus sharing the host kernel; hypervisor configuration still requires hardening. |
| GPU, DRM/KMS, Wayland, USB, or special hardware testing | Dedicated hardware or explicitly configured full VM with passed-through devices | Generic microVM profiles may not expose the needed devices; passing devices through adds risk. |
| Synology NAS | External read-only monitoring first; optional vendor-supported container later | Keep DSM's vendor-managed boot, storage, and update lifecycle untouched. |
| Windows/macOS guest | Separate feasibility and licensing/hardware gate | Do not infer feasibility or licensing from Linux-guest support. |

No single isolation mechanism is right for every workload. Containers share the host kernel and generally start with lower overhead; VMs have a separate kernel and support guest-kernel tests. The correct choice depends on threat model, needed devices, guest OS, durability, and measurable performance.

## Cross-OS support: keep two matrices

Support needs two distinct records:

- **Host support matrix:** which OS release/hardware profiles can run each Luminous component, and which exact capabilities have passed.
- **Guest/workload matrix:** which OCI architectures and base images, or which VM guest OS/image versions, have been built and tested.

A workload running inside an Ubuntu VM on NixOS is evidence that the guest image and VM path work. It is **not** evidence that sovereign-boot, Nixward, or the native Luminous host integration supports Ubuntu directly.

Likewise, running a container on NixOS is not evidence that its kernel, GPU, systemd, or DSM integrations work on every host. Report per-profile evidence, with image digest, host/guest versions, architecture, VMM/runtime, test harness, and exact result.

## Performance and correctness benchmark

Use the same hardware, workload, and test harness for each candidate. Report raw samples and p50/p95, not a single best-case number.

- Container cold start and warm restart.
- MicroVM and full-VM cold boot to declared readiness.
- Idle RAM floor and incremental memory per additional guest.
- Steady-state CPU overhead, disk throughput/latency, and network throughput/latency.
- Maximum stable concurrent workloads under a declared CPU/RAM budget.
- Crash/restart behavior, update rollback, host reboot, and failure isolation.
- Build repeatability, artifact hashes, booted guest identity, and post-start health.

Do not publish "blazing fast" or "better than X" without a reproducible baseline and named machine, versions, workload, and confidence/variance information. Hardware virtualization (KVM/VT-x/AMD-V), virtio configuration, storage, memory pressure, and host kernel configuration materially affect results.

## Safety architecture

The NixOS host owns the declared baseline. The workload controller receives narrow typed capabilities instead of shell/root access.

- Separate observation, planning, validation, authorization, apply, verification, and recovery.
- Bind an authorized plan to the exact host state, guest image digest, config digest, resource limits, network policy, and target.
- Never pass through `/var/run/docker.sock`, the Incus admin socket, arbitrary host paths, raw devices, or unrestricted `/dev/kvm` access to Symthaea or a general control agent.
- VMM/container launch requests must be allowlisted and policy-checked by a small privileged broker.
- Default network ingress-deny; expose only declared ports and destinations. Avoid host networking for multi-tenant workloads.
- Give stateful workloads named storage, backups, restore verification, and explicit deletion semantics.
- A failed observer or cognitive process must not stop normal host boot. Authorization/evidence failures block mutations.
- Treat a timeout during create/start/stop as indeterminate until observed state resolves it; never blindly repeat a non-idempotent operation.

## Proposed milestones

1. **Inventory:** document hardware virtualization availability and the current NixOS host's memory/storage/network budgets. No machine changes.
2. **OCI baseline:** create one harmless, non-root test service with pinned digest, read-only filesystem where possible, resource limits, no privileged mode, and a bounded health check.
3. **MicroVM proof:** run one minimal, non-sensitive NixOS guest with pinned configuration, no host-store write access, explicit networking, and a verified clean shutdown/restart.
4. **Cross-OS proof:** launch an official, pinned Ubuntu Server image in an isolated KVM/QEMU test network. Verify SSH/login readiness, package provenance, shutdown, image identity, and host survival; no host bootloader changes.
5. **Benchmarks:** compare container, microVM, and full VM under the exact same workload; capture evidence before choosing defaults.
6. **Control-plane contract:** implement observation-only inventory first. Add mutations only after policy, authorization, independent verification, and failure/recovery tests are in place.
7. **Host distro expansion:** continue Ubuntu native-host validation, then Fedora and Debian. The host support sequence remains separate from guest VM availability.
8. **Optional scale track:** evaluate Incus or Firecracker only for a concrete need not covered by the first stack.

## Research references

- [NixOS Manual: VM versus container trade-offs](https://nixos.org/manual/nixos/stable/)
- [NixOS system configuration and declarative flakes](https://wiki.nixos.org/wiki/NixOS_system_configuration)
- [microvm.nix: backends, shares, storage, and network constraints](https://github.com/microvm-nix/microvm.nix)
- [Podman Quadlet: declarative systemd-managed containers](https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html)
- [Incus: multi-distribution system containers and VMs](https://github.com/lxc/incus)
- [Incus security model and privileged API boundary](https://github.com/lxc/incus/blob/main/doc/explanation/security.md)
- [Firecracker design and production jailer guidance](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md)

The source links are architectural references; they are not evidence that Luminous Platform already implements or qualifies these runtimes.
