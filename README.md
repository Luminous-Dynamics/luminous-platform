# Luminous Platform

The **Luminous Platform** is the infrastructure and operating environment for Luminous-Dynamics AI systems. It provides a sovereignty-first, fail-open, consciousness-aware runtime for NixOS hosts, with a planned path toward explicitly qualified support for additional Linux distributions.

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                   COGNITIVE CORE                        │
│  symthaea · symtropy · kosmic-lab                       │
│  ~1.7M LOC Rust · HDC + LTC + Active Inference         │
└──────────────────────┬──────────────────────────────────┘
                       │ state / receipts / intent
┌──────────────────────▼──────────────────────────────────┐
│                  PLATFORM LAYER  ◄── YOU ARE HERE       │
│                                                         │
│  ┌──────────────────┐  ┌──────────────────┐            │
│  │  sovereign-boot  │  │    nixward       │            │
│  │  Boot animation  │  │  NixOS mgmt +   │            │
│  │  + state machine │  │  machine contracts│           │
│  │  + recovery      │  │  + warded nodes  │            │
│  └──────────────────┘  └──────────────────┘            │
│  ┌──────────────────┐  ┌──────────────────┐            │
│  │  sovereign-ops   │  │  luminous-edge   │            │
│  │  Fleet intent /  │  │  Meta-flake:     │            │
│  │  ops console     │  │  composes all ↑  │            │
│  └──────────────────┘  └──────────────────┘            │
└──────────────────────┬──────────────────────────────────┘
                       │ PQC transport · consent ledger
┌──────────────────────▼──────────────────────────────────┐
│                  NETWORK LAYER                          │
│  xenia-peer · xenia-wire · mycelix mesh                 │
└─────────────────────────────────────────────────────────┘
```

## Platform Components

| Repo | Role | Flake? | Status |
|------|------|--------|--------|
| [sovereign-boot](https://github.com/Luminous-Dynamics/sovereign-boot) | Boot animation (DRM/KMS), fail-open state machine, Linux recovery executor | ✅ | Qualified (v0.3.4f) |
| [nixward](https://github.com/Luminous-Dynamics/nixward) | Conscious NixOS management, machine contracts, warded node security | ✅ | Active |
| [sovereign-ops](https://github.com/Luminous-Dynamics/sovereign-ops) | Fleet intent, ops console, Nixward receipt verification | 🔒 private | Incubating |
| [luminous-edge](https://github.com/Luminous-Dynamics/luminous-edge) | Meta-flake composing the platform layer | ✅ | v0.1 |
| [xenia-peer](https://github.com/Luminous-Dynamics/xenia-peer) | Sovereign operations daemon (H.264, PQC handshake, consent ledger) | ✅ | Active |
| [xenia-wire](https://github.com/Luminous-Dynamics/xenia-wire) | Wire protocol | ✅ | Active |

## Safety Contracts

Every platform component follows the same core safety rule:

> **Platform components may observe host state; they must never be required for host boot.**

- `sovereign-boot` — disabled by default; QEMU-gated before any host enable
- `nixward` — read-only diagnosis by default; actuation requires explicit operator receipt
- `sovereign-ops` — intent-only; never directly mutates NixOS configuration

## Cross-Distribution Roadmap

NixOS remains the reference integration. Additional distribution support is a planned, capability-by-capability effort—not a claim that the current releases already support every Linux distribution.

The proposed **native host integration** sequence is Ubuntu Server 24.04 LTS in QEMU → Ubuntu 26.04 LTS / desktop qualification → Fedora → Fedora bootc as a separate image-based profile → Debian Stable. In parallel, the proposed workload strategy is to keep NixOS as the preferred declarative host for machines we control, use OCI containers for suitable services, and use KVM-backed VMs for guest OS compatibility and stronger isolation. Begin with Podman/OCI plus a minimal microvm.nix proof for NixOS guests; use a general KVM/QEMU manager for heterogeneous guest OSes. This is a proposed architecture, not a measured performance claim or implemented runtime.

Synology DSM is a separate NAS appliance track: first, a remote read-only SNMPv3 monitor using Synology's documented MIBs; optionally, a narrow DSM Web API adapter if required; then, an unprivileged Container Manager/Compose profile for one explicitly qualified model. It is not treated as generic Linux support. None of these profiles touches DSM boot/recovery, installs a privileged SPK, or gives a control-plane agent Docker-socket/root authority. The proposed monitor contract is [profiles/synology-dsm-snmpv3.yaml](profiles/synology-dsm-snmpv3.yaml). See [docs/SYNOLOGY_MONITOR_ADR.md](docs/SYNOLOGY_MONITOR_ADR.md) for the Rust client evaluation and fail-closed collector design. For the host/guest distinction and placement policy, see [Host and Workload Strategy](docs/HOST_AND_WORKLOAD_STRATEGY.md). The [workload profile schema](schemas/workload-profile.schema.json) and [example profiles](profiles/workloads/) now define the first machine-readable contract; CI validates them and rejects selected unsafe configurations.

See [Cross-Distribution Support Strategy](docs/CROSS_DISTRO_SUPPORT.md) for the support matrix, architecture boundaries, staged acceptance gates, and required evidence. Distribution support will be reported per capability and exact tested profile; a queued workflow is not a pass.

## Quick Start: NixOS Integration

Pin the entire platform layer through a single flake input:

```nix
# /etc/nixos/flake.nix
inputs.luminous-edge.url = "github:Luminous-Dynamics/luminous-edge";

# In your NixOS configuration
imports = [
  inputs.luminous-edge.nixosModules.sovereignBoot
  inputs.luminous-edge.nixosModules.nixward
];
```

Or pin components individually:
```nix
inputs.sovereign-boot.url = "github:Luminous-Dynamics/sovereign-boot";
inputs.nixward.url         = "github:Luminous-Dynamics/nixward";
```

## Version Matrix

| luminous-edge | sovereign-boot | nixward | sovereign-ops | nixpkgs |
|---------------|----------------|---------|---------------|---------|
| v0.1 | v0.3.4f (605735c) | main (Aug 2026) | incubating | nixos-unstable |

## License

AGPL-3.0-or-later. Commercial licensing available — see [COMMERCIAL_LICENSE.md](COMMERCIAL_LICENSE.md).
