# ADR 0002: Holochain as a decentralized trust and coordination plane

**Status:** Proposed for review  
**Date:** 2026-10-09  
**Scope:** Luminous Platform, Mycelix, Sovereign Ops, Xenia, and evidence exchange  
**Decision owner:** Platform maintainers

## Decision summary

**Yes, use Holochain—but do not make it the platform's universal database or execution authority.**

Adopt Holochain selectively for multi-party records where independently operated agents need to publish, validate, retain, and verify shared statements under common application rules. Integrate this capability through Mycelix and a narrow platform adapter.

Keep PostgreSQL as the authoritative system of record for the first shared service's operational case state, SLA/queue calculations, transactional inbox/outbox, and business idempotency. Keep Xenia responsible for consent-bound remote sessions and its own capability/execution checks. Keep Nixward responsible for supported NixOS machine transactions and recovery. Symthaea may interpret evidence and propose plans but cannot grant authority.

Holochain records can express claims and validation outcomes. They do not, by themselves, prove a real-world statement is true, establish universal consensus over a single global current state, or authorize a privileged host action.

## Why Holochain adds value here

Holochain is a good fit when a record is jointly relevant to multiple independently operated organizations and no one organization should be the exclusive custodian of the shared history. Relevant examples include:

- A customer and MSP exchanging an incident handoff, acknowledgement, or dispute record.
- Multiple organizations attesting to an evidence manifest, provenance chain, acceptance, or correction.
- Federated identity, membership, delegated trust credentials, and shared governance using the relevant Mycelix domains.
- Cooperative/supply-chain records where multiple parties need local agency and validation rules without giving a central platform operator unilateral control over all shared records.
- Durable provenance pointers and attestations that can be checked independently of the operator console.

Holochain data validation is especially useful for rejecting structurally or semantically invalid DHT operations using deterministic integrity-zome rules. Validation should be narrowly defined and thoroughly tested with multi-agent integration tests, not inferred from single-agent Rust unit tests.

## System-of-record boundary

| Data/capability | Owner | Holochain role |
|---|---|---|
| Canonical operational case lifecycle, current priority/assignee, SLA clocks, technician queue | PostgreSQL-backed service-management plane | Optional references or agreed cross-organization projections only |
| Inbox, idempotency, provider mappings, transactional outbox | PostgreSQL-backed integration service | Not the transactional inbox or outbox |
| Cross-organization acknowledgements, attestations, disputes, agreed policies, shared provenance | Mycelix / selected Holochain hApp | Primary shared record where decentralization adds value |
| Confidential raw evidence, screen/audio, secrets, customer payloads | Tenant-controlled encrypted evidence/object store and access-control service | Store only minimal, classified references/commitments when appropriate; not raw material by default |
| Consent-bound interactive remote session and actual remote input/action | Xenia | No Holochain substitute for live session authorization |
| NixOS change, activation, recovery, postcondition receipt | Nixward | Optional verifiable reference/attestation; Nixward retains action authority |
| Diagnosis, risk estimate, recommendations, outcome analysis | Symthaea | May consume validated records, but its output is advice, not permission |
| Operator/customer experience and stable external API | Leptos clients and documented REST/OpenAPI/CloudEvents APIs | Holochain client integration is one adapter, not the only API |

Do not mirror every PostgreSQL row into a DHT. That would create duplicated source-of-truth and conflict-resolution problems without automatically creating a better product.

## The first Holochain pilot

Build one deliberately narrow **cross-organization incident acknowledgement and evidence-attestation flow** after the synthetic case API and evidence-reference semantics exist.

1. A synthetic case is created and updated in the PostgreSQL service.
2. The service creates a versioned, minimized share package with a tenant-authorized set of fields, a stable case reference, evidence-manifest digest, data-classification label, and explicit retention/access policy.
3. A reviewed adapter publishes a corresponding record to a dedicated Mycelix/Holochain hApp. The record identifies the author and schema version and contains only approved information; it must not include a ticket description, credentials, raw diagnostics, screen/audio, or personal data by default.
4. A separately controlled customer or partner agent can acknowledge, attest, dispute, or supersede that record according to deterministic integrity rules.
5. The adapter reconciles observed Holochain records into a read model and appends an internal activity entry. It never overwrites canonical case state simply because a DHT record arrived later.
6. A reviewer can inspect the record/provenance and distinguish a cryptographically valid, rule-valid attestation from a verified real-world outcome. The system preserves the identity, trust basis, scope, revocation/freshness status, and limitations of each claim.

The first pilot should be read/attest-only: no live ticket writes back to the provider, no remote execution, and no billing or settlement.

The initial machine-readable package boundary now lives in [Holochain Attestation Share Package V1](../../contracts/holochain-attestation-v1/README.md), with its [JSON Schema](../../contracts/holochain-attestation-v1/share-package.schema.json), synthetic fixture, and adversarial tests. The companion `validator.py` adds cross-field checks for unique manifest references and self-referential dispute/supersession targets, which JSON Schema alone does not fully express. It is a payload contract only; it does not implement the Holochain adapter or integrity zome and has not yet been qualified against multi-agent DHT behavior.

## Integration protocol and correctness

- Treat the PostgreSQL outbox as the durable source for dispatch requests to Holochain. Give every publication a stable idempotency key derived from the local operation's durable identity; record publication attempts and resulting DHT action/entry references.
- Treat Holochain publication as asynchronous and potentially unavailable. Expose pending, published/observed, rejected, conflicted, and retry-required states distinctly. Do not show a green verified status because a message was queued or a zome call returned successfully.
- Use reconciliation to compare local publication intents with observed records and retry safely. Do not claim that a PostgreSQL transaction and DHT publication are one atomic distributed transaction.
- Do not infer event order from wall-clock timestamps or agent IDs. Define explicit causality/supersession rules per record type, and expose unresolved dependencies and competing claims.
- Version DNA/zome schemas and validation rules deliberately. Test upgrade/migration and compatibility between participating agents before rolling out changes.
- Keep public records data-minimized. Holochain private entries keep entry content private on the author's source chain, but associated action metadata can still be published; therefore "private entry" must not be treated as metadata privacy or equivalent to an encrypted confidential datastore.
- Use explicit capability grants for access to zome functions and reviewed identity/membership policy, but never treat a Holochain capability grant or historical attestation as a live authorization to execute a privileged operation. Xenia/Nixward must check current authority, target, scope, expiry, revocation, preconditions, and verified postconditions at their own execution boundary.

## Threat and privacy model

Before any customer data is published, document what other peers can learn from record content, action metadata, links, timing, agent identity, record size, and correlation with external information. Hashes of predictable or low-entropy content can leak information by guessing; a digest is not a privacy control by itself.

Classify each field as public, partner-shared, tenant-private, secret, or regulated. Require an explicit data-sharing decision for public/partner-shared records, and prefer random opaque identifiers over customer names, email addresses, ticket subjects, hostnames, and globally stable person identifiers. Raw evidence stays in an access-controlled store; share signed manifests or encrypted payload references only where access, retention, key rotation, and revocation are designed.

A DHT validation result means the record passed those code-defined rules under the validation dependencies available to peers. It is not an independent audit of the author's device, a guarantee every peer has seen the record, proof that the attested event happened, or proof a system was remediated safely.

## Availability and operations

Holochain does not eliminate operational infrastructure. Bootstrap and signalling services help agents discover and connect; peers can be offline and other agents' data may be temporarily unavailable. The supported product must specify:
- Which operations work from PostgreSQL and tenant-local data if Holochain is unavailable.
- Which cross-party attestations require current network access and must remain pending/fail closed when not reachable.
- Redundant and monitored bootstrap/signalling infrastructure, supported conductor versions, backup/recovery, identity/key recovery, capacity/load tests, and hApp/DNA upgrade procedures.
- The retention and redaction model for local source chains, DHT records, cached evidence, and exported audit packages.

Avoid routing the critical path for local ticket viewing, lawful data export, safe host boot, or an already-authorized local recovery action through the DHT. Where current multi-party approval is a prerequisite to a high-impact action, unavailability must block that action explicitly rather than silently bypassing the requirement.

## Release and qualification policy

The repository must pin a mutually compatible Holochain conductor, HDK, HDI, client, Lair, and hApp toolchain. Do not independently float these dependencies. The upstream releases page currently lists 0.8.0 development prereleases and 0.7.1 release candidates; do not treat a development/RC tag as a production release. Re-evaluate the pinned set only after a stable compatible release and tested upgrade path are available.

Before calling the integration qualified, require:

- [ ] Multi-agent tests with separately provisioned agent identities and real conductor/DHT validation.
- [ ] Positive and negative integrity-zome tests for membership, schema, transition/attestation rules, revocation, conflicts, and malformed dependencies.
- [ ] Tests where dependent DHT data is missing or unavailable, ensuring unresolved validation is not misreported as successful validation.
- [ ] PostgreSQL-outbox crash/restart tests across publication attempts, lost acknowledgements, duplicate retries, and delayed DHT visibility.
- [ ] Cross-tenant and cross-organization tests proving that unauthorized records, links, evidence pointers, and exports are not disclosed or accepted.
- [ ] Privacy review of entry content and action metadata using synthetic adversarial identifiers and correlation scenarios.
- [ ] DNA/zome upgrade compatibility and recovery tests across the pinned conductor matrix.
- [ ] Conductor/DHT availability tests with peers offline, bootstrap/signalling disruption, delayed propagation, and conflicting or superseding records.
- [ ] An evidence report tied to the exact code, DNA hash, dependency lockfile, test corpus, and observed results.

Unit-test count or successful packing of a hApp is not evidence that the distributed behavior has passed these gates.

## Existing Luminous reuse and maturity

Prefer integrating through the existing Mycelix domain and the mycelix-leptos-client only where its browser transport and conductor compatibility are verified for the supported deployment. Do not invent a second generic Holochain SDK inside the service-management repository.

The Mycelix repository's current README describes the overall system as pre-alpha and explicitly distinguishes unit-test coverage from multi-agent DHT tests and deployed UI maturity. Therefore, begin this platform pilot only after selecting a narrow, mature-enough hApp slice and verifying its actual conductor/test compatibility. This ADR does not promote all Mycelix domains to production readiness.

## References

- [Holochain application architecture](https://developer.holochain.org/concepts/2_application_architecture/): DNA-level modularity and agent-centric app networks.
- [Holochain DHT model](https://developer.holochain.org/concepts/4_dht/): distributed storage, public DHT entries, source chains, and metadata visibility.
- [Holochain validation](https://developer.holochain.org/build/validation/): deterministic integrity rules and dependency behavior.
- [Holochain capabilities](https://developer.holochain.org/build/capabilities/): zome-call capability grants and revocation.
- [Operating a hApp](https://developer.holochain.org/build/operating-a-happ/): availability, always-on nodes, bootstrap, signalling, and WebRTC/STUN considerations.
- [Holochain releases](https://github.com/holochain/holochain/releases): evaluate stable releases separately from prereleases.
- [Luminous Platform ADR 0001](./0001-production-operational-state-store.md): PostgreSQL system-of-record decision.
- [Mycelix maturity matrix](https://github.com/Luminous-Dynamics/mycelix): verify domain-specific maturity rather than assuming the entire project is production-ready.

## Related limitations

This ADR is a design proposal. It does not implement a production hApp, an adapter, cross-organization trust governance, or the qualification evidence above.
