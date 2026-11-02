# Connectivity Policy Core

Pure Rust decision core for evaluating connection candidates against a declared service intent and hard user policy. This crate has no runtime dependencies and performs no privileged network operations.

## Scope

The caller supplies link observations such as availability, authorization state, observation age, data-metering status, estimated cost in a stated currency, p95 latency, reliability, throughput, remaining allowance, energy cost and declared trust class. The caller also supplies an intent:

- `LocalOnly`
- `RealtimeInteractive`
- `Messaging`
- `GeneralBrowsing`
- `BackgroundSync`
- `BulkTransfer`
- `SensitiveData`

The function applies hard exclusions, then ranks eligible candidates using explicit intent-specific lexicographic criteria. The result includes the selected link, ordered alternatives, ranking criteria, per-link exclusions and an explicit no-route outcome. Candidate input order is not used to break ties; stable link ID is the final tie-break.

## Use

From the repository root:

```sh
cargo test --manifest-path tools/connectivity-policy-core/Cargo.toml
cargo clippy --manifest-path tools/connectivity-policy-core/Cargo.toml --all-targets -- -D warnings
```

A minimal use example:

```rust
use connectivity_policy_core::{
    choose_link, LinkCandidate, SelectionPolicy, TrafficIntent,
};

let policy = SelectionPolicy::permissive(TrafficIntent::BackgroundSync, "ZAR");
let decision = choose_link(&policy, &observed_links)?;
println!("selected={:?}", decision.selected_link_id);
println!("excluded={:?}", decision.assessments);
# Ok::<(), connectivity_policy_core::DecisionError>(())
```

The sample "permissive" constructor is for experimentation, not a recommended production policy. Set explicit max cost, max latency, minimum reliability/throughput/trust, observation age and allowance requirements for the actual service contract.

## Non-negotiable design rules

- Cost caps, authorization, trust, metering, latency, reliability, capacity and observation freshness are eligibility constraints; score cannot buy its way around them.
- Cost comparisons use minor units per GiB in a specified currency. Cross-currency amounts are never compared as if they were equivalent.
- `SensitiveData` requires the candidate to be labelled `Trusted`, regardless of the lower policy minimum. This is a local routing classification, not proof of end-to-end encryption or endpoint trust.
- A `LocalOnly` intent accepts a declared local-service path without Internet availability. Other intents require Internet availability.
- Unknown/stale observations do not pass a hard bound. If no route meets the user's hard requirements, the result explicitly contains `NoEligibleCandidate`.
- The algorithm does not infer intent from traffic content. Adapters should pass only the minimum link metadata required for the policy.
- The `authorized` field is an assertion supplied by an adapter and must only be set after it checks applicable provider, device and legal permissions.
- Recommendations may become stale immediately. A privileged adapter must re-check live link state and policy authorization before changing routes.

## Tests authored

Thirteen unit tests cover cheaper-link selection, a hard cost cap, missing/mismatched price data, stale observations, local-only use without WAN, sensitive-intent trust, authorization, input-order-independent ranking, explicit no-route outcomes, unknown metered allowance, invalid currency and duplicate link identifiers.

These tests are source code, not passing test evidence. Compilation, Cargo.lock generation, unit-test execution and Clippy must run in a Rust-enabled environment before integration. This crate is not yet an OS routing adapter, eSIM controller, carrier network selector, phone app, emergency-calling implementation or production service.
