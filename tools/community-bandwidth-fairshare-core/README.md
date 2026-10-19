# Community bandwidth fair-share reference core

Pure Rust, standard-library-only reference model for dividing a caller-supplied byte budget across peers on a shared community link.

## Contract

- The caller supplies a measured capacity budget for one explicit scheduling window.
- Each peer has an opaque identifier, relative weight, essential demand and ordinary demand.
- The policy reserves a portion of capacity for essential demand. If essential demand exceeds the reserve, it may consume the remaining capacity before ordinary traffic is granted.
- If all essential demand is met, unused reserve is borrowable by ordinary traffic rather than being stranded.
- Weighted max-min filling respects per-peer demand caps and redistributes unused shares.
- Integer remainder bytes are allocated deterministically after canonical peer-ID sorting.
- The result reports both classes' grants and any unallocated capacity.

The crate uses integer byte quantities and 128-bit intermediate products. It rejects zero capacity, a reserve larger than capacity, duplicate or malformed IDs, invalid weights, and input above 4,096 peers.

## Build and check

From the repository root:

    cargo fmt --manifest-path tools/community-bandwidth-fairshare-core/Cargo.toml -- --check
    cargo test --manifest-path tools/community-bandwidth-fairshare-core/Cargo.toml --all-targets
    cargo clippy --manifest-path tools/community-bandwidth-fairshare-core/Cargo.toml --all-targets -- -D warnings
    cargo run --release --manifest-path tools/community-bandwidth-fairshare-core/Cargo.toml --bin fairshare-benchmark

The benchmark emits CSV for 10, 100, 1,000 and 4,096 peers. Results vary by runner and must not be promoted as universal latency claims.

## Non-goals and trust boundaries

This is not a Wi-Fi airtime scheduler, AP traffic shaper, router configuration tool, admission-control system, identity provider, cryptographic verifier or proof of network fairness. A byte budget is not the same as radio airtime or successful application work. Caller-supplied capacity, peer weights, essential labels, and identity bindings are external assertions.

Peer IDs must be opaque and must not encode a person's name, phone number, email, hardware address or other direct identifier. The syntax check does not prove privacy or prevent Sybil identities. Deployments need authenticated principals, an anti-Sybil/admission policy, measurement provenance, and adapter-level enforcement.

See ../../docs/COMMUNITY_BANDWIDTH_FAIRSHARE_V1.md for the algorithm, qualification boundaries and benchmark plan.
