# MCP Tool Contract Integrity Benchmark v0.1

**Status:** synthetic descriptor/authority contract only. No production MCP server, real credential or live invocation is used or claimed.

## Purpose

Test the connector trust boundary before CAPC evaluates realized effects. A friendly tool name or human-readable description is not a stable capability identity.

## Pinned descriptor

The reviewed descriptor binds server identity/origin, protocol revision/transport, tool name and server-issued tool id, description digest, input-schema digest, output-schema digest, complete annotations digest, canonical descriptor digest, server artifact digest, build provenance and canonicalization revision.

The canonical descriptor commitment is a single identity across all reviewed components. A change in any component, server origin/id or build identity requires a new review and a grant bound to the new identity. A previously reviewed digest does not transfer.

The synthetic digest-shaped values exercise exact matching; they are not hashes of real MCP descriptors. A runtime implementation must define canonical JSON/CBOR normalization, hash the complete descriptor and independently reproduce the digest.

## Per-invocation authority

Each call binds descriptor digest and tool id to the exact invocation/effect tuple and checks:
- issuer equals the expected authorization server;
- token audience is the receiving MCP server;
- granted scopes include the required tool/resource scopes and no wildcard overgrant;
- policy epoch is current;
- nonce is fresh and not already consumed;
- session/state handles are not accepted as authentication.

Descriptor integrity, authority integrity, owner admission and observed effect remain separate outcomes.

## Frozen panel

14 synthetic cases cover input/output schema drift, annotation changes, origin substitution, server build substitution, tool-id shadowing, wrong token audience, wrong issuer, wildcard scopes, nonce replay, session-handle misuse, stale approval after descriptor replacement, unresolved server identity and a fully pinned positive control.

## Targets (not results)

- 100% deterministic detection of all descriptor component changes.
- Zero invocations admitted under a descriptor digest different from the reviewed digest.
- Zero cross-audience/cross-origin token acceptance.
- Zero replay or session-handle-only authorization.
- 100% allowed positive controls.
- Report false acceptance/rejection, tool utility and latency separately; no composite score.

## Ownership and claim ceiling

Consumes CARI's capability surface, CAPC effect tuple, Mycelix grants/authority, endpoint ordered trace and independent effect oracle. No new MCP proxy, tool registry, authorization engine or runner. A passing checker qualifies synthetic mutations only; real MCP security needs pinned client/server/protocol versions, exact descriptor capture, real per-request authorization verification and independent resource-side effect evidence.
