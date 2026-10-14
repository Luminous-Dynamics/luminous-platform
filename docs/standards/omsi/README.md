# Open Managed Services Interoperability (OMSI)

**Status:** pre-RFC discussion draft · **Version:** 0.1 · **Date:** 2026-10-12  
**Governance:** not yet established · **Name clearance:** not performed

OMSI proposes a vendor-neutral contract for exchanging managed-service operations and their evidence across MSP platforms, customer environments and specialist providers. It is a proposal—not an adopted industry standard, a certification, or a claim that the Luminous projects currently provide a complete production MSP stack.

## The gap this proposal targets

MSP environments combine service desks/PSA, RMM agents, endpoint and cloud control planes, identity, backup, security operations, networking and specialist providers. Integrations may exchange records or commands without a shared cross-vendor contract for tenant/resource binding, exact authorization, operation state, independent postcondition verification and evidence portability.

OMSI proposes to standardize that boundary and reuse existing domain standards instead of replacing them.

## Documents

- [Pre-RFC proposal](RFC-PROPOSAL-0001.md)
- [Threat model](THREAT-MODEL.md)
- [Scope and gap register](SCOPE-AND-GAP-REGISTER.md)
- [Research notes](RESEARCH-NOTES.md)
- [IT Service Core prior-art mapping](profiles/it-service-core/0.1/PRIOR-ART-MAPPING.md)
- [Field Service profile for trades and asset maintenance](profiles/field-service/0.1/PROFILE.md)
- [Nixward adapter mapping](profiles/nixos/0.1/NIXWARD-ADAPTER-MAPPING.md)
- [Local conformance report and limits](CONFORMANCE-REPORT.md)

## Architecture in one sentence

Every consequential operation should be bound to an authenticated tenant, stable target, immutable manifest, bounded grant over that exact manifest, explicit execution lifecycle, and evidence for any claimed postcondition.

## Reference implementation mapping (not requirements)

- **Luminous Platform:** integration boundary and service workflows.
- **Nixward / Luminous Nix:** first candidate executor for a narrow NixOS change profile.
- **Symthaea:** advisory diagnostics and recommendations; never an authority primitive.
- **Mycelix:** optional trust/federation experiment only after identity, delegation, revocation and recovery are validated.
- **Xenia:** optional remote-support adapter; not a required dependency.

A conforming implementation may use other languages, databases, operating systems, transports, identity systems and execution tools. No Luminous product, blockchain, Holochain, AI system or central SaaS service is required.

## Review focus

1. Is operation/authority/evidence a useful small interoperability kernel distinct from existing ITSM/RMM and field-service schemas?
2. Are the proposed records minimal enough to map across independently developed systems?
3. What belongs in Core 0.1 versus a service-specific profile?
4. Which independent MSPs, tradespeople, service providers, customers and security reviewers should review the problem statement before stable schemas are frozen?

## Non-claims

Passing draft schemas or local semantic tests does not verify cryptographic signatures, prove target observations true, establish tenant isolation in a live deployment, demonstrate cross-provider interoperability, or certify a service. The field-service profile is an incubation proposal; see the scope register and threat model.
