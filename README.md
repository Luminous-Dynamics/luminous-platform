# Luminous Platform

The **Luminous Platform** is the infrastructure and operating environment for Luminous-Dynamics AI systems. It provides a sovereignty-first, fail-open, consciousness-aware runtime for NixOS hosts.

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

## Architecture RFCs

- [Xenia Operations Fabric](docs/XENIA_OPERATIONS_FABRIC_RFC.md) — proposed boundaries and integration plan for a sovereign, AI-native MSP/service-management platform, including a ConnectWise-first adoption path. This is a design proposal, not a production-readiness claim.
- [Unified Platform Product and UI Decision](docs/UNIFIED_PLATFORM_PRODUCT_AND_UI_DECISION.md) — recommendation to build toward a broader sovereign operations platform, phase the ConnectWise alternative, and adopt Leptos for new Rust-native operator/customer web surfaces without coupling the platform API to the frontend framework.

## Safety Contracts

Every platform component follows the same core safety rule:

> **Platform components may observe host state; they must never be required for host boot.**

- `sovereign-boot` — disabled by default; QEMU-gated before any host enable
- `nixward` — read-only diagnosis by default; actuation requires explicit operator receipt
- `sovereign-ops` — intent-only; never directly mutates NixOS configuration

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
