# Artifact Fetch Policy

This is an experimental, dependency-free Rust library of deterministic policy
primitives for a future artifact-fetch broker. It is deliberately **not** an
HTTP client or a complete SSRF defense.

## What this crate checks

- Exact administrator-owned HTTPS-origin allowlisting, after conservative host
  canonicalization.
- Empty DNS answer rejection and fail-closed validation of every answer in a
  set. One disallowed address rejects the entire set.
- A `ValidatedResolution` value binds the validated IP set to the exact
  normalized origin that produced it; peer verification rejects reuse for a
  different origin.
- Actual socket-peer membership in that origin-bound validated set.
- Redirect target authorization on every hop, a configurable hop cap, and no
  credential forwarding when the normalized origin changes.
- Declared content-length and streamed-byte limits.

## Required transport contract

The transport that eventually uses this library must:

1. Parse and validate artifact URIs with a dedicated, maintained URL parser
   before constructing an HTTPS origin. This crate accepts host and port
   separately; it does not parse URLs.
2. Resolve DNS after authorizing the origin, pass all A and AAAA answers to
   authorize_resolution, and connect directly to one of the returned
   addresses. Preserve the original hostname for TLS SNI, certificate
   verification, and HTTP Host. Do not make an unchecked second DNS lookup.
3. Re-run destination authorization for every redirect, retry, fallback
   address, and reconnect. Disable redirects by default unless each hop is
   explicitly authorized. Do not inherit ambient proxy settings.
4. Enforce check_received_bytes as data streams in and abort as soon as the
   limit is exceeded. A post-download size check alone is insufficient.
5. Download into a broker-owned staging file with safe file handling, verify
   the expected content digest, then atomically promote only the fully verified
   artifact into a trusted cache. Digest verification and safe file promotion
   are outside this crate.
6. Enforce network egress policy outside the process as defense in depth. This
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

Passing these tests proves only that this library's deterministic policy
checks behave as tested. It does not qualify any actual transport or artifact
download.
