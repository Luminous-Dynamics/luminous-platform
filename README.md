# Luminous Platform

**Sovereign NixOS operations with explicit authority, verifiable changes, and recoverable system state.**

Luminous Platform is a composable infrastructure layer for NixOS hosts. It brings together system observation, machine contracts, controlled change workflows, boot lifecycle components, and integration modules.

The design goal is straightforward: make consequential system changes inspectable and recoverable without making the platform itself a prerequisite for the host to boot.

> **Maturity note:** This repository is an architecture and integration overview, not a claim that every component is production-ready or independently security-audited. Check each component repository for its current release, qualification gates, tests, known limitations, and threat model.

## What the platform provides

| Component | Role | Current documentation status |
|---|---|---|
| [Nixward](https://github.com/Luminous-Dynamics/nixward) | NixOS observation, machine contracts, diagnostics, and evidence-bound system-change workflows | Active engineering; read-only diagnosis by default, consequential actuation requires explicit authorization |
| [Sovereign Boot](https://github.com/Luminous-Dynamics/sovereign-boot) | Boot lifecycle state, framebuffer animation, and Linux recovery components | Opt-in and disabled by default; follow repository-specific QEMU and host-enablement gates |
| [Luminous Edge](https://github.com/Luminous-Dynamics/luminous-edge) | Meta-flake composing supported platform modules | Public integration flake; check its release and locked inputs |
| [Nix Signature Policy](https://github.com/Luminous-Dynamics/nix-signature-policy) | Reference policy model for multi-authority and downgrade-resistant Nix signature authorization | Separate adjacent project; not implied to be enabled by Luminous Edge |
| [Xenia](https://github.com/Luminous-Dynamics/xenia-peer) | Consent-first peer-to-peer remote sessions and operator authorization | Related pre-alpha project; not a production remote-administration dependency of the base platform |

The fleet-operations workstream is not part of the public quick-start contract. Consult the public component repositories for the currently available integration surface.

## Architecture

```text
                  Operator intent
                        |
                        v
              +-------------------+
              |     Nixward       |
              | observe / diagnose|
              | plan / validate   |
              | authorize / apply |
              | verify / recover  |
              +---------+---------+
                        |
             explicit operator authority
                        |
                        v
              +-------------------+
              |   NixOS host      |
              | configuration     |
              | system state      |
              | generations       |
              +---------+---------+
                        |
             boot-time lifecycle state
                        |
                        v
              +-------------------+
              |  Sovereign Boot   |
              | state / recovery  |
              +-------------------+

       Luminous Edge composes the public NixOS modules.
       Xenia and Nix Signature Policy are adjacent projects,
       not mandatory dependencies of the base host.
```

## Safety and authority boundaries

The platform follows this invariant:

> **Platform components may observe host state; they must never be required for the host to boot.**

Component-specific boundaries:

- **Nixward:** diagnosis is read-only by default. A model recommendation, cognitive signal, or natural-language request is advisory; it is not authorization. Consequential changes follow an evidence-bound transaction lifecycle.
- **Sovereign Boot:** disabled by default. Do not enable on a physical host unless the repository's QEMU and host-qualification gates for the exact revision have passed.
- **Luminous Edge:** composition does not remove the need to review, pin, and test each component version.
- **Xenia:** pre-alpha. Its working demonstrations are not a security audit or a production-readiness claim.

Nixward documents the transaction sequence as:

```text
observe -> plan -> validate -> authorize
        -> snapshot -> apply -> verify
        -> promote or recover
```

The authorization step must be explicit and evidence-bound. Passing tests, generated plans, or a high model-confidence score must never silently grant host-mutation authority.

## Quick start

The public integration entry point is [Luminous Edge](https://github.com/Luminous-Dynamics/luminous-edge).

Add it as a flake input:

```nix
{
  inputs.luminous-edge.url = "github:Luminous-Dynamics/luminous-edge";
}
```

Import the modules exposed by the flake in the NixOS configuration:

```nix
{ inputs, ... }:

{
  imports = [
    inputs.luminous-edge.nixosModules.nixward
    inputs.luminous-edge.nixosModules.sovereignBoot
  ];

  # Sovereign Boot remains disabled unless its qualification gates
  # for the exact revision and target host have passed.
}
```

For production or shared infrastructure:

1. Review the source repository and its security notes.
2. Pin the exact input revisions in `flake.lock`; do not rely on a moving branch.
3. Start with observation and read-only diagnostics.
4. Exercise changes and recovery in disposable VMs.
5. Require explicit operator authorization for consequential changes.
6. Retain the validation output and exact revision used for the deployment.

You can also pin [Nixward](https://github.com/Luminous-Dynamics/nixward) or [Sovereign Boot](https://github.com/Luminous-Dynamics/sovereign-boot) individually. Follow each repository's own setup instructions.

## Evaluation and qualification

The platform should be evaluated on operational outcomes, not feature count alone.

| Area | Evidence to collect |
|---|---|
| Change safety | Unauthorized-action negative controls; policy-denial behavior; no silent actuation |
| Reproducibility | Exact flake inputs, lockfile, build outputs, and repeatable system generations |
| Recovery | VM fault-injection results, rollback success, and recovery time |
| Drift detection | Detection precision/recall against labeled configuration changes |
| Reliability | Successful and failed transitions, with failure classification |
| Operator efficiency | Time and effort per correctly completed change |
| Supply-chain policy | Downgrade, signer-diversity, revocation, and rollback test vectors |

A benchmark report should identify hardware, Nixpkgs revision, component revisions, workloads, failure cases, and limitations. Do not describe a queued workflow or unreviewed experiment as a passing qualification.

## Related projects

- [Symthaea](https://github.com/Luminous-Dynamics/symthaea) — experimental cognitive architecture. Any future advisory integration must remain separate from authorization.
- [Mycelix](https://github.com/Luminous-Dynamics/mycelix) — decentralized identity, provenance, and coordination research.
- [Xenia](https://github.com/Luminous-Dynamics/xenia-peer) — pre-alpha consent-first remote-session application.
- [Nix Signature Policy](https://github.com/Luminous-Dynamics/nix-signature-policy) — separate Nix authorization-policy research and conformance suite.

These relationships describe the portfolio direction. They do not imply that all components are installed, integrated, or supported by the public Luminous Edge flake.

## Licensing and commercial use

Licenses are component-specific; inspect the license in each source repository and crate before redistributing or incorporating it. The Luminous Platform repository provides a [commercial licensing contact](COMMERCIAL_LICENSE.md) for inquiries concerning covered components. Do not assume that one repository's license applies to every dependency or sibling project.

## Contributing

Start with the component repository closest to the issue:

- [Luminous Platform](https://github.com/Luminous-Dynamics/luminous-platform)
- [Luminous Edge](https://github.com/Luminous-Dynamics/luminous-edge)
- [Nixward](https://github.com/Luminous-Dynamics/nixward)
- [Sovereign Boot](https://github.com/Luminous-Dynamics/sovereign-boot)
- [Nix Signature Policy](https://github.com/Luminous-Dynamics/nix-signature-policy)

Prefer small, reproducible changes with explicit failure cases and negative controls. Security-sensitive changes should include the threat-model update and independent review appropriate to their impact.
