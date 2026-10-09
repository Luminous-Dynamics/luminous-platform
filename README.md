# Luminous Platform

The **Luminous Platform** is the infrastructure and operating environment for Luminous-Dynamics AI systems. It provides a sovereignty-first, boot-safe runtime with bounded authority and evidence-producing operations for NixOS hosts.

See [Global Deployment and Business Model](docs/GLOBAL_DEPLOYMENT_AND_BUSINESS_MODEL.md) for the proposed product boundaries, deployment profiles, business model, and qualification roadmap. That document is a roadmap, not a claim of production readiness or universal compliance.

For the partner/operator on-ramp, start with [Start a Luminous Business](docs/START_A_LUMINOUS_BUSINESS.md) and the [MSP Incident-to-Evidence starter pack](business-packs/msp-incident-to-evidence/README.md). The pack includes a service manifest and reusable intake, scope, and delivery-report templates. Its first delivery mode is manual-first; it does not claim a qualified live PSA connector.

See [One Luminous Platform, Many Business Packs](docs/PLATFORM_AND_REPOSITORY_STRATEGY.md) for the product/repository strategy. The goal is one coherent platform with independently qualified vertical packs—not a separate platform for every industry.

For the proposed globally portable, member-oriented buying network and free business capability floor, see [Global Cooperative Commerce](docs/GLOBAL_COOPERATIVE_COMMERCE.md). The first structural prototype is the [Cooperative Offer schema](schemas/cooperative-offer-v1.schema.json), with synthetic fixtures and regression tests in the [Cooperative Buying pack](business-packs/cooperative-buying/README.md). These are proposed structures, not a live purchasing service or universal-compliance claim.

For a machine-readable draft, see the [Deployment Profile Contract](docs/DEPLOYMENT_PROFILE_CONTRACT.md), [JSON Schema](schemas/deployment-profile-v1.schema.json), and [local NixOS pilot example](profiles/examples/local-nixos-pilot.yaml). A first declaration-only validator is available at `tools/validate_deployment_profile.py`; it has a regression corpus in `tests/test_deployment_profile_validator.py` and a pinned dependency file.

```bash
python -m pip install --requirement requirements-profile-validator.txt
python -m unittest discover -s tests -p 'test_deployment_profile_validator.py' -v
python tools/validate_deployment_profile.py profiles/examples/local-nixos-pilot.yaml
```

The validator checks syntax and declaration consistency only. Its `VALID_DECLARATION` result does not mean that a target was tested or is production-ready; any profile that claims runtime qualification remains blocked pending adapter-specific evidence evaluation.

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

Boot availability and authorization are separate concerns: optional platform services should not prevent local boot or recovery, but an unknown, stale, or unverifiable authority must never grant privileged actuation.

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
