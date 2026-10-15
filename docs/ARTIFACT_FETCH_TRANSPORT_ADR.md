# Artifact Fetch Transport ADR

**Status:** Proposed implementation decision; not yet implemented or qualified  
**Date:** 2026-10-15  
**Scope:** Future artifact-fetch broker used by workload-profile/runtime adapters

## Context

`tools/artifact-fetch-policy` currently provides deterministic policy primitives only. It does not resolve DNS, open sockets, perform TLS, send HTTP requests, stream response bodies, hash artifacts, or write files. These are deliberate boundaries; unit tests for policy helpers are not end-to-end SSRF or downloader qualification.

The next milestone must put the checks on the actual network path. In particular, the transport must not resolve a hostname again after authorization, silently follow redirects, inherit an environment proxy, auto-decompress a response before accounting for its size, or send request credentials before peer/TLS validation.

## Decision

Build a small, HTTPS-only Rust transport with **Hyper HTTP/1.1 over an explicit TCP dialer and rustls TLS**, rather than using a high-level client as the final enforcement boundary.

The current crate research (2026-10-15) found:

- Reqwest 0.13.5 documents per-domain `resolve_to_addrs`, `no_proxy`, configurable redirect behavior, body-decompression controls, and total/read timeouts. Those are useful controls, and Reqwest remains a viable option for non-adversarial fetching.
- Hyper exposes an HTTP/1 connection handshake over an IO stream. That lets the broker own TCP dialing and verify the actual `TcpStream::peer_addr()` before TLS and before any HTTP request bytes are sent.
- Hyper-Rustls demonstrates the standard Hyper/Tokio/Rustls integration pattern. The implementation should retain the requested hostname as the TLS server name/certificate-validation identity while dialing the validated IP address directly.
- Hickory Resolver 0.26.3 provides an asynchronous resolver API and dual-stack IP lookup. DNS answers still must flow through `FetchPolicy::authorize_resolution`; the resolver is not itself the authorization policy.

References reviewed:
- Reqwest ClientBuilder: https://docs.rs/reqwest/latest/reqwest/struct.ClientBuilder.html
- Hyper HTTP/1 handshake: https://docs.rs/hyper/latest/hyper/client/conn/http1/fn.handshake.html
- Hyper client connector architecture: https://hyper.rs/guides/client/configuration/
- Hyper-Rustls connector source: https://docs.rs/hyper-rustls/latest/src/hyper_rustls/connector.rs.html
- Hickory Resolver: https://docs.rs/hickory-resolver/latest/hickory_resolver/

These are design references, not a claim that their dependencies are already pinned or installed. Before the implementation lands, select exact compatible versions, verify their MSRVs against the repository's Rust toolchain, commit the generated `Cargo.lock`, and run formatting, locked tests, and dependency-audit checks in CI.

## Required request lifecycle

1. Parse the artifact URI with a maintained URL parser. Require `https`, a valid host, no userinfo, and no unsupported URL components. Construct the normalized `HttpsOrigin` and authorize it before DNS or socket activity.
2. Establish one monotonic total deadline for the whole fetch. DNS lookup, each connect, TLS handshake, response headers, redirects, body streaming, and retries all consume the same budget. Per-phase budgets may shorten it but must never restart it.
3. Resolve every required A/AAAA result for the named host using the selected resolver. Pass the complete answer set to `authorize_resolution`, failing closed if any answer is disallowed. For an IP-literal origin, use only that literal.
4. Dial one validated `SocketAddr` directly. Do not pass the hostname to a second resolver or allow a proxy to make the destination connection. Read the socket's actual peer address and call `verify_connected_peer` before performing TLS or emitting HTTP bytes. If a candidate fails, only try another address from the same validated set and within the original deadline.
5. Start TLS with rustls using the URL's original hostname/IP as the verification identity and SNI where applicable. Never replace the certificate-validation name with the selected IP for a DNS hostname. Use normal certificate and hostname validation; no insecure-verifier switches.
6. Initially use one HTTP/1.1 connection per hop and no pooling. This is intentionally less efficient than pooling but reduces cross-origin reuse/coalescing complexity until the full invariants have regression coverage. Disable automatic redirects, transparent decompression, ambient proxy configuration, and implicit retries. Advertise `Accept-Encoding: identity`.
7. Parse HTTP with Hyper; do not implement an HTTP parser manually. Inspect all `Content-Encoding` and `Content-Length` field occurrences before processing the body. Use `validate_content_encoding_headers` and `validate_content_length_headers`. Let the HTTP parser enforce message framing, and explicitly reject invalid `Transfer-Encoding`/`Content-Length` combinations or parser errors.
8. For each redirect, resolve a relative `Location` against the current URL with the URL parser, require HTTPS, and call `authorize_redirect_hop` on the same mutable `RedirectChain`. Re-resolve and revalidate the destination before connecting. Never forward credentials across origins; do not treat a rejected hop as recoverable within the same chain.
9. For every response body data frame, account for the chunk using the policy-bound `DownloadBudget` before writing it. Abort immediately on a size violation, timeout, parser error, or unsupported encoding. A declared `Content-Length` is an early check, never a replacement for streamed accounting.
10. Write only to a broker-owned staging location opened without following attacker-controlled symlinks. Compute the expected digest while streaming, compare it after clean EOF, sync the staged file as required, and promote atomically only after the digest matches. A partial, timed-out, truncated, or over-limit download must never appear in the trusted cache.
11. Record structured, redacted evidence: policy/origin identifier, chosen address family, redirect count, byte count, elapsed time, expected/observed digest, and terminal reason. Never log credentials, secret URL parameters, authorization headers, or body data.

## Initial non-goals

- No HTTP/2 or HTTP/3, connection pooling, proxy support, ambient credentials, automatic retries, content decoding, or arbitrary methods in the first transport milestone.
- No production use of URL-provided file paths or profile-provided output paths.
- No weakening of the public-address policy to make local integration tests convenient. Test-only loopback endpoints must be isolated behind test-only wiring, never an allowlist shipped in production.
- No claim of complete SSRF defense from an in-process client alone; the runtime also needs OS/network egress controls appropriate to its deployment.

## Acceptance gates

- Unit tests for URL/authority parsing, redirect-chain transitions, all response-header occurrences, content-encoding rejection, missing/oversized lengths, cumulative budget overrun/overflow, and terminal failure states.
- Deterministic local TLS test fixtures whose test CA and certificates are checked into the test corpus or generated reproducibly. No test may disable hostname/certificate verification to get a pass.
- Integration tests prove the connected peer equals a member of the exact validated answer set and port, a mismatched/IP-literal/loopback target is rejected before any HTTP request bytes are written, and TLS still validates the original hostname.
- Integration tests cover denied cross-origin redirects, redirects beyond the cap, attempts to reuse a poisoned chain, slow/stalled body reads, overall deadline exhaustion across multiple hops, duplicate/malformed length fields, compressed response rejection, chunked-body size enforcement, truncated framing, and digest mismatch.
- Oversized or failed artifacts are never promoted; successful artifacts are promoted atomically only after length and digest checks.
- Exact-head CI must pass `cargo fmt --check`, `cargo test --locked` (including transport integration tests), schema/profile validation, and a dependency audit. Queued or cancelled runs do not qualify.
- Security tests run repeatedly with stable outcomes; test fixtures use no public network and no external artifact downloads.

## Rollout

1. Keep the current dependency-free policy crate as the first layer.
2. Add the concrete transport in a separate Rust module/crate with exact versions and a committed lockfile. Prefer a small API accepting a validated fetch request, policy, and expected digest, returning a receipt—not exposing arbitrary client configuration.
3. Implement HTTP/1.1 one-hop fetching plus deterministic tests before redirects or filesystem promotion.
4. Add stateful redirects and per-hop DNS/peer verification.
5. Add secure staging, streaming hashing, and atomic promotion.
6. Wire a runtime adapter only after transport CI passes on the exact commit and review confirms that the transport applies every policy decision.
