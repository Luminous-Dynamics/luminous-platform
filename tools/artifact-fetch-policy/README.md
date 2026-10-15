# Artifact Fetch Policy

This is an experimental, dependency-free Rust library of deterministic policy
primitives for a future artifact-fetch broker. It is deliberately **not** an
HTTP client or a complete SSRF defense.

## What this crate checks

- Exact administrator-owned HTTPS-origin allowlisting, after conservative host
  canonicalization.
- Empty DNS answer rejection and fail-closed validation of every answer in a
  set. One disallowed address rejects the entire set. IP-literal origins must
  resolve to exactly that literal, not an arbitrary public address.
- A `ValidatedResolution` value binds the validated IP set to the exact
  normalized origin that produced it; peer verification rejects reuse for a
  different origin.
- Actual socket-peer IP membership and exact origin-port matching.
- Rejection of ambiguous legacy IPv4 hostname spellings (including shortened,
  octal, and hexadecimal forms that URL parsers can interpret differently).
- A private-state, non-cloneable redirect-chain tracker bound to its original
  allowlist. It counts every authorized hop, rejects caller-supplied counters,
  and becomes terminal after a denied hop; no credential forwarding occurs
  when the origin changes.
- Declared content-length and streamed-byte limits, plus a policy-bound cumulative byte meter that becomes terminal after a limit breach or counter overflow. Counter overflow has its own error so it cannot be misreported as an ordinary size breach.
- Fail-closed response content-encoding validation: accept only an absent header or `identity` unless a future transport explicitly meters compressed wire bytes and decoded artifact bytes.
- Required positive connect, read-idle, and whole-fetch time budgets, with phase timeouts constrained by the total budget.

## Required transport contract

The transport that eventually uses this library must:

1. Parse and validate artifact URIs with a dedicated, maintained URL parser
   before constructing an HTTPS origin. This crate accepts host and port
   separately; it does not parse URLs.
2. Resolve DNS after authorizing the origin, pass all A and AAAA answers to
   authorize_resolution, and connect directly to one of the returned
   addresses at the origin's exact port. Before sending HTTP data, pass the
   actual socket peer IP and port to verify_connected_peer. Preserve the
   original hostname for TLS SNI, certificate verification, and HTTP Host. Do
   not make an unchecked second DNS lookup.
3. Start a `RedirectChain` at the authorized origin and call
   `authorize_redirect_hop` for each response redirect. Keep the same mutable
   chain for the entire fetch; it owns the hop counter and is not cloneable.
   A denied redirect poisons the chain, so the HTTP transaction must abort.
   Re-run destination authorization for every retry, fallback address, and
   reconnect as well. Disable redirects by default unless each hop is
   explicitly authorized. Do not inherit ambient proxy settings.
4. Enforce the connect and read-idle timeouts on the actual transport, plus one
   absolute total-fetch deadline spanning all redirects and retries. Do not
   restart the total budget for each attempt. These values are policy
   configuration only; this crate does not interrupt network I/O.
5. Request `Accept-Encoding: identity` and use `validate_content_encoding`
   on the response before processing the body. This default rejects gzip,
   Brotli, and other compressed bodies so a small encoded length cannot hide a
   large decoded artifact. A future transport that supports compression must
   cap both wire bytes and decoded bytes independently.
6. Create a `DownloadBudget` from the policy and call `record_chunk` for each
   body chunk before accepting/writing it. Abort immediately on error. The meter
   uses checked cumulative arithmetic, has a distinct counter-overflow error,
   and remains terminal after the first over-limit event, so callers cannot
   accidentally resume after rejection. A post-download size check alone is
   insufficient.
7. Download into a broker-owned staging file with safe file handling, verify
   the expected content digest, then atomically promote only the fully verified
   artifact into a trusted cache. Digest verification and safe file promotion
   are outside this crate.
8. Enforce network egress policy outside the process as defense in depth. This
   helper does not control routing, DNS, TLS, proxies, redirects, sockets, or
   filesystem writes.

## Address-range policy

The implementation conservatively permits only IPv4 addresses outside its
curated special-purpose deny prefixes, and IPv6 within global-unicast
2000::/3 excluding selected transition, documentation, IETF-assignment, and
SRv6 SID (`5f00::/16`) ranges. Some entries intentionally block entire registered prefixes even when
an exception inside the prefix is globally reachable. This is a conservative
default, not an authoritative replacement for a current network-policy engine
or cloud/organization-specific deny lists.

The ranges must be reviewed against the current
[IANA IPv4 Special-Purpose Address Registry](https://www.iana.org/assignments/iana-ipv4-special-registry),
[IANA IPv6 Special-Purpose Address Registry](https://www.iana.org/assignments/iana-ipv6-special-registry),
and [IANA IPv6 Global Unicast Address Space](https://www.iana.org/assignments/ipv6-unicast-address-assignments)
as part of routine maintenance. Network-specific internal, service, VPN,
overlay, and metadata destinations may require additional explicit blocks.

The policy is informed by the
[OWASP SSRF Prevention Cheat Sheet](https://cheatsheetseries.owasp.org/cheatsheets/Server_Side_Request_Forgery_Prevention_Cheat_Sheet.html):
domain allowlisting alone is insufficient; the connected address must be one
of the addresses that were validated, and the original hostname must still be
used for TLS certificate verification.

## Test

No third-party crates are used.

    cargo fmt --check --manifest-path tools/artifact-fetch-policy/Cargo.toml
    cargo test --locked --offline --manifest-path tools/artifact-fetch-policy/Cargo.toml

The suite now contains 18 deterministic unit tests. Passing them proves only
that this library's policy checks behave as tested. It does not qualify any
actual transport or artifact download.
