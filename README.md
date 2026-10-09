# Luminous Platform

The **Luminous Platform** is experimental infrastructure and an operating environment for Luminous-Dynamics systems on NixOS. It is designed to preserve host sovereignty while keeping optional platform components out of the boot-critical path. Its failure semantics are deliberately split: optional boot integrations preserve host availability, while privileged mutation authority fails closed when target identity, pre-state, operator authorization, or required evidence cannot be validated. Cognitive signals and natural-language intent are advisory; they are never authorization by themselves.

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
| [sovereign-boot](https://github.com/Luminous-Dynamics/sovereign-boot) | Boot animation (DRM/KMS), fail-open state handling, Linux recovery executor | ✅ | Qualified (v0.3.4f) |
| [nixward](https://github.com/Luminous-Dynamics/nixward) | NixOS management, machine contracts, warded-node security | ✅ | Active |
| [sovereign-ops](https://github.com/Luminous-Dynamics/sovereign-ops) | Fleet intent, ops console, Nixward receipt verification | 🔒 private | Incubating |
| [luminous-edge](https://github.com/Luminous-Dynamics/luminous-edge) | Meta-flake composing the platform layer | ✅ | v0.1 |
| [xenia-peer](https://github.com/Luminous-Dynamics/xenia-peer) | Sovereign operations daemon (H.264, PQC handshake, consent ledger) | ✅ | Active |
| [xenia-wire](https://github.com/Luminous-Dynamics/xenia-wire) | Wire protocol | ✅ | Active |

## Safety Contracts

Every platform component follows the same availability rule:

> **Platform components may observe host state; they must never be required for ordinary host boot.**

- `sovereign-boot` — disabled by default; QEMU-gated before any host enable.
- `nixward` — read-only diagnosis by default; actuation requires explicit operator authorization.
- `sovereign-ops` — intent-only; never directly mutates NixOS configuration.

## Trust and Evidence Boundaries

These are intended architectural contracts. The readiness and qualification status of each component must still be checked against that repository's current evidence; this overview is not proof that every component has satisfied every contract.

- **Boot availability:** Optional components may observe or annotate host boot, but must not become a prerequisite for ordinary boot. Fail-open behavior here protects availability; it does not grant privileged authority.
- **Advisory cognition:** Symthaea outputs, Phi values, confidence scores, and natural-language plans may inform a decision, but do not authorize a machine mutation.
- **Privileged changes:** Nixward's documented transaction lifecycle is `observe → plan → validate → authorize → snapshot → apply → verify → promote/recover`. A consequential change must not proceed when the exact target identity, pre-state, operator authorization, or required evidence is missing or mismatched. See the [system transaction architecture](https://github.com/Luminous-Dynamics/nixward/blob/main/docs/SYSTEM_TRANSACTION_ARCHITECTURE.md).
- **Qualification evidence:** Results should bind to the exact commit being evaluated. A queued, skipped, cancelled, missing, or pre-job-failed workflow is not a pass. Automated scanner output is useful evidence, but is not by itself an independent security audit or production-security certification.

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

The component revisions below are the revisions locked by the current [luminous-edge flake.lock](https://github.com/Luminous-Dynamics/luminous-edge/blob/main/flake.lock), not a promise that every combination has been independently tested.

| luminous-edge | sovereign-boot | nixward | sovereign-ops | nixpkgs |
|---------------|----------------|---------|---------------|---------|
| v0.1 | [v0.3.4f (`605735c`)](https://github.com/Luminous-Dynamics/sovereign-boot/commit/605735ceb6d9adf4bf64ee6a7d47cdd5d734381a) | [`ae60256`](https://github.com/Luminous-Dynamics/nixward/commit/ae60256f0be73138a539ecda048e81056dd14a30) | Incubating | [`151fa4e`](https://github.com/NixOS/nixpkgs/commit/151fa4e8ddfdd8dd25d945ad94ed54a13de9f6e4) |

## License

AGPL-3.0-or-later. Commercial licensing available — see [COMMERCIAL_LICENSE.md](COMMERCIAL_LICENSE.md).
