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

Use NixOS for host configuration and use OCI images for services that do not need their own kernel. Begin with Podman and rootless containers where feasible; manage persistent services declaratively and avoid mutating containers as a substitute for versioned images. Pin image digests and resource/permission policy. Rootless alone does not establish strong isolation: still use least privilege, namespaces, seccomp/LSM policy when available, bounded mounts, and explicit network policy. The v1alpha1 profile contract currently permits only the runtime-default seccomp profile: custom policies remain rejected until the profile can bind them to an immutable, digest-identified artifact and a runtime adapter can prove it applied that exact artifact.

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

## Further native-host candidates

After the first Ubuntu server and cross-OS guest gates, prioritize targets by distinct integration value rather than raw OS count:

| Candidate | Why it adds coverage | Required independent profile |
|---|---|---|
| Debian Stable 13 (Trixie) | A conservative Debian-family host target with its own release/update cadence and package/service lifecycle. Debian's release documentation identifies Trixie as the stable release; pin the exact point-release image for each run. | Debian base image digest, architecture, package manifest, service behavior and upgrade test. |
| Fedora Server | Exercises RPM/DNF and SELinux while remaining a conventional host. | Fedora release/image digest, enforcing SELinux mode, permissions/labels, systemd lifecycle, and RPM packaging evidence. |
| Rocky Linux 10 | Adds a long-lifecycle RHEL-compatible host profile materially different from Fedora's faster-moving release. Rocky's [current release table](https://docs.rockylinux.org/latest/releases/) lists 10.2 as the current minor, with general support through 31 May 2030 and end of life/security support through 31 May 2035. Only the latest minor is supported, so qualification must record the exact minor and be re-evaluated when it changes. | Exact minor release, architecture, DNF/RPM behavior, SELinux enforcement, security policy, and update maintenance window. |
| AlmaLinux 10 | A second Enterprise Linux compatibility target only if users, CI, or a product requirement need it. | Its own image/digest and package/kernel/security behavior; do not inherit Rocky's support result. |
| openSUSE MicroOS | Tests a transactional, read-only-oriented host model. Upstream's transactional-update design applies changes to a new snapshot and switches on reboot, with rollback semantics; this changes update and recovery handling. | Image/snapshot identity, pending versus booted deployment identity, reboot/promotion, health confirmation, rollback, and separation of app/config/user data. |

MicroOS must not use the same mutation path as Ubuntu or Rocky. Its integration should observe the candidate snapshot, authorize the exact transaction, verify the actually booted snapshot after reboot, and record an indeterminate state whenever promotion or rollback has not been proven. References: [openSUSE MicroOS](https://microos.opensuse.org/) and [transactional-update](https://github.com/openSUSE/transactional-update).

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
- **Mount paths are not capabilities.** OCI volumes and microVM shares must identify their source with an opaque `sourceId` and a bounded in-container/guest target; the profile must never supply an arbitrary host source path. For OCI volumes, `source` identifies the source class (`dedicated_state`, `readonly_config`, or `readonly_artifact`), while `sourceId` selects one concrete source in an administrator-owned registry. A `sourceId` is a lookup key, not permission to create or widen a grant. Unknown IDs, duplicate source IDs, overlapping/duplicate targets, unexpected access modes, or a pre-state/config digest mismatch must fail closed.
- Resolve host sources relative to a pre-opened registry root, not the process working directory. On Linux, use a directory file descriptor and `openat2(2)` resolution constraints such as `RESOLVE_BENEATH | RESOLVE_NO_MAGICLINKS`; require `RESOLVE_NO_SYMLINKS` for source classes whose contract forbids symlinks. Keep and use the validated file descriptor so a path is not re-resolved after validation. If the platform/kernel cannot enforce the selected policy, classify the operation as unsupported and block it—do not fall back to a vulnerable `canonicalize-then-open` check. See the [Linux openat2(2) manual](https://man7.org/linux/man-pages/man2/openat2.2.html).
- Treat mount target syntax validation as hygiene only: the schema rejects lexical traversal/root targets, but cannot prove runtime path resolution or containment. Resolve targets within the container/guest root using the runtime's safe mechanisms; after start, verify the effective mount table against the exact authorized set and check that required read-only mounts are actually read-only. Never infer successful mounting from a successful start response alone.
- For virtiofs or similar microVM shares, run the sharing service with least privilege and expose only the registered directory and authorized mode. Do not expose the host root, Nix store, runtime sockets, device trees, or broad home-directory trees by default.
- Default network ingress-deny; expose only declared ports and destinations. Avoid host networking for multi-tenant workloads.
- Treat network peer declarations as policy intent, not network enforcement. Each rule now requires `peerType` (`hostname`, `ipv4`, or `ipv6`), `peer`, and a human-readable justification; for ingress, the peer is the source, and for egress, the peer is the destination. Wildcards are rejected, dotted-numeric strings cannot be mislabeled as hostnames, and IPv4/IPv6 literals are checked with the validator's format checker. Because JSON Schema does not require `format` assertions by default, the Python harness also self-tests that its checker rejects malformed IPv4 and IPv6 values before accepting profiles; any alternate implementation must provide equivalent fail-closed format enforcement. A future runtime adapter must govern DNS separately, validate every resolved IPv4/IPv6 address against the permitted peer policy, bind connections to validated addresses, and re-check retries and redirects. Block loopback, link-local, private, and metadata-service destinations unless a separately authorized exception explicitly requires one. Hostname allowlisting alone does not prevent DNS rebinding; see the [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html). Keep allowlisted-egress workloads unqualified until enforcement and failure-path tests prove this boundary.
- **Artifact acquisition is a separate privilege boundary.** A content digest identifies expected bytes; it does not authorize the destination from which bytes are fetched. OCI image references must name an explicit registry and repository rather than rely on an implicit default registry. The privileged broker must enforce administrator-owned allowlists for registries and VM-artifact origins, and it must treat `artifact://` as an opaque lookup into an administrator-owned artifact catalog—not as an arbitrary network URL. HTTPS VM image locators must not embed userinfo, query strings, or fragments; they are restricted to printable ASCII and reject backslashes and percent-encoding to avoid parser disagreements; avoid signed URLs and credentials in workload profiles. This strict syntax gate is deliberate because Python's [URL parsing APIs do not validate URLs](https://docs.python.org/3/library/urllib.parse.html#url-parsing-security). Resolve and validate every A/AAAA address before connecting; reject loopback, link-local, private, unspecified, multicast, and metadata-service destinations unless a separately reviewed policy explicitly authorizes the exact case. Connect to the validated address while retaining certificate hostname verification/SNI. Do not inherit ambient proxy settings. Disable redirects by default; if a registry or artifact store requires redirects, validate every hop independently against the same destination policy, re-resolve the destination, strip credentials when authority changes, and bound redirect count, response size, and time. Verify OCI manifest/index and referenced blob digests before use, and verify a VM image's declared SHA-256 before promoting a completed staged artifact into the trusted cache; partial or mismatched downloads must be discarded. These controls follow the content-addressing model in the [OCI Distribution Specification](https://specs.opencontainers.org/distribution-spec/) and the destination-validation guidance in the [OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html). Profile validation alone does not implement or prove any of these network controls.
- Give stateful workloads named storage, backups, restore verification, and explicit deletion semantics.
- A failed observer or cognitive process must not stop normal host boot. Authorization/evidence failures block mutations.
- Treat a timeout during create/start/stop as indeterminate until observed state resolves it; never blindly repeat a non-idempotent operation.

## Machine-readable profile contract

A first draft contract now lives at [`schemas/workload-profile.schema.json`](../schemas/workload-profile.schema.json), with three schema fixtures under [`profiles/workloads/`](../profiles/workloads/): OCI container, NixOS microVM, and Ubuntu full VM. The fixtures deliberately use example-only image/artifact identifiers and must not be launched or fetched.

The schema constrains common workload properties (identity/qualification, allowed host and architecture, runtime kind, resource budgets, network policy, secrets by reference, lifecycle authorization, and evidence status). OCI volumes and microVM shares use opaque source IDs; OCI source IDs are required alongside a source class, and never carry a host path. JSON Schema itself enforces that `hostDataAccess=none` forbids OCI volumes and microVM shares, while `declared_mounts_only` requires a declared volume/share for those workload kinds; the validator repeats these invariants as defense in depth. The validator also rejects duplicate source IDs, duplicate or nested mount targets, duplicate normalized network-rule identities, and conflicting OCI host-port/protocol bindings. Each published OCI port must declare both the container port and the host port; `exposure` specifies the required binding scope (`loopback_only` or `private_network`), and the runtime must verify the actual socket/listener binding rather than infer it from the requested profile. The schema also rejects inconsistent network policy/rule pairs, writable mounts sourced from read-only configuration/artifact categories, root mount targets, repeated-slash paths, and `.`/`..` path segments. Normal absolute targets remain valid; lexical validation is hygiene, not a host containment boundary. The validator checks cross-field invariants that plain JSON Schema cannot express, including VM vCPU/RAM/disk budgets and limiting the initial NixOS microVM profile to the NixOS reference host. It makes the first OCI path require digest-pinned images, non-root UID/GID, read-only root filesystem, no-new-privileges, no privileged mode, no host namespaces, an explicit seccomp policy, bounded resources, and no public port exposure. VM examples require a digest-identified guest artifact and an isolated network.

[The profile validator](../tools/validate_workload_profiles.py) checks the schema, validates all examples, and uses negative cases to prove that selected dangerous configurations are rejected. A dedicated GitHub Actions workflow runs those checks on matching pull-request and main-branch changes. The schema also binds qualification labels to evidence: an `example_only` profile must remain `design_only` with no passed-test claims; `ci_verified` requires a matching status, at least one passed test, and a receipt reference; `hardware_qualified` requires matching status, passed tests, and receipt references. The validator additionally requires every ID in `testsPassed` to exist in `testsRequired`, so a receipt cannot claim an undeclared check. This is evidence-presence and cross-field consistency checking, not independent proof that a referenced test was actually performed. Schema validation does not start runtimes, verify the VMM, or qualify performance/security on a real host. A CI result counts as passed only after the workflow completes successfully.

## Proposed milestones

1. **Contract and CI:** validate the profile schema and its positive/negative fixtures in CI. This contract stage exists in the proposal branch; wait for the CI result and fix issues before calling it passed.
2. **Inventory:** document hardware virtualization availability and the current NixOS host's memory/storage/network budgets. No machine changes.
3. **OCI baseline:** create one harmless, non-root test service with pinned digest, read-only filesystem where possible, resource limits, no privileged mode, and a bounded health check.
4. **MicroVM proof:** run one minimal, non-sensitive NixOS guest with pinned configuration, no host-store write access, explicit networking, and a verified clean shutdown/restart.
5. **Cross-OS proof:** launch an official, pinned Ubuntu Server image in an isolated KVM/QEMU test network. Verify SSH/login readiness, package provenance, shutdown, image identity, and host survival; no host bootloader changes.
6. **Benchmarks:** compare container, microVM, and full VM under the exact same workload; capture evidence before choosing defaults.
7. **Control-plane contract:** implement observation-only inventory first. Add mutations only after policy, authorization, independent verification, and failure/recovery tests are in place.
8. **Host distro expansion:** continue Ubuntu native-host validation, then Fedora and Debian. The host support sequence remains separate from guest VM availability.
9. **Optional scale track:** evaluate Incus or Firecracker only for a concrete need not covered by the first stack.

## Research references

- [NixOS Manual: VM versus container trade-offs](https://nixos.org/manual/nixos/stable/)
- [NixOS system configuration and declarative flakes](https://wiki.nixos.org/wiki/NixOS_system_configuration)
- [microvm.nix: backends, shares, storage, and network constraints](https://github.com/microvm-nix/microvm.nix)
- [Podman Quadlet: declarative systemd-managed containers](https://docs.podman.io/en/latest/markdown/podman-systemd.unit.5.html)
- [Incus: multi-distribution system containers and VMs](https://github.com/lxc/incus)
- [Incus security model and privileged API boundary](https://github.com/lxc/incus/blob/main/doc/explanation/security.md)
- [Firecracker design and production jailer guidance](https://github.com/firecracker-microvm/firecracker/blob/main/docs/design.md)

The source links are architectural references; they are not evidence that Luminous Platform already implements or qualifies these runtimes.
