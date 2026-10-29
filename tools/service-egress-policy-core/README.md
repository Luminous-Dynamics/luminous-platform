# Service-aware egress policy core v1

This crate is a dependency-free Rust reference model for deciding whether an agent may request one broker-defined service operation using one opaque credential handle.

## What it does

- Requires an externally pinned policy digest and minimum accepted epoch.
- Fails closed on missing/stale/reversed clocks, missing or contradictory observations, unknown authentication/revocation state, rollback, expiry, and malformed policy structure.
- Binds one atomic request identity to a target, subject image digest, service ID, operation ID, and optional credential handle.
- Requires a single capability to cover the service, operation, subject, target, and credential handle together. Grants are never unioned across capabilities.
- Uses canonical opaque identifiers, not free-form URLs, HTTP headers, paths, bodies, or credential bytes.
- Returns an authorization receipt bound to the exact request identity and a half-open expiry time.
- Selects among equivalent eligible capabilities deterministically, independent of policy vector ordering.

The service and operation identifiers must be registered by a separate trusted broker. For example, a broker may map service "code-host" and operation "repo.read_file" to a fixed host, TLS identity, HTTP method, route template, and response limit. The policy core does not perform that mapping.

## What it does not do

This is **not an egress proxy or an enforcement mechanism**. It does not:

- remove direct network access from an agent process;
- open or restrict sockets, verify TLS, resolve DNS, or normalize arbitrary URLs;
- prove the adapter's Verified declaration true;
- verify policy signatures, persist rollback state, or maintain a replay/revocation ledger;
- keep real secrets out of process memory by itself; or
- independently observe packet, HTTP, credential, or durable-state effects.

Those are separate requirements for the supervised broker and OS boundary. Until direct egress is blocked outside the process and independently observed, a decision from this crate cannot be described as network enforcement.

## Why the inputs are opaque IDs

A free-form host, URL, path, HTTP header, or method string is too easy to normalize inconsistently across layers. The core accepts canonical broker-registry identifiers only. The broker owns the exact mapping from an operation ID to endpoint, method, route template, TLS identity, destination resolution, response bounds, and credential use. Redirects must produce a fresh normalized request and undergo another authorization decision. A caller must not reuse a prior receipt for a redirected request.

Credential identifiers are handles, not secrets. A broker resolves a handle outside the agent process and binds the resulting credential to the exact allowed service and operation.

## Test coverage

The authored Rust tests cover canonical IDs, exact scope, subject/target mismatch, service and operation mismatch, credential-handle mismatch, credential-free operations, no capability union, missing/unverified/contradictory evidence, policy digest and rollback checks, policy/capability revocation states, validity windows, malformed policy, stable behavior under capability order changes, and the claim boundary around redirect hops.

## Next milestone

Build a supervised Linux broker with:
1. no direct network path from the agent workload;
2. a service registry that maps operation IDs to fixed endpoints and method/route contracts;
3. credentials kept outside the workload and exposed only through broker calls;
4. redirect re-authorization and DNS/TLS/peer binding;
5. packet-level and server-side effect oracles; and
6. adversarial tests for DNS rebinding, IP literals, IPv6, redirect chains, proxy bypass, secret leakage, replay, and revocation races.

Do not call this crate production-ready until those enforcement and observation layers exist.
