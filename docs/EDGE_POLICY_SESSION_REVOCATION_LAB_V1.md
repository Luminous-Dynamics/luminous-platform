# Luminous Edge existing-session revocation lab v1

**Evidence level:** opt-in, virtual Linux-kernel namespace integration test. Not physical AP/firewall qualification, vendor comparison, or authenticated evidence.

## Why this benchmark exists

A new-flow allow/deny matrix does not establish what happens to an already-established TCP stream after a policy update. In stateful firewalls, generic `established,related` rules may continue accepting packets for a conntracked flow even when the rule that admitted new connections has been removed. The [nftables conntrack state documentation](https://wiki.nftables.org/wiki-nftables/index.php/Matching_connection_tracking_stateful_metainformation) defines the established state and its matching behavior.

This lab deliberately distinguishes three outcomes instead of conflating them:

| Case | Policy transition | Existing TCP/8080 session | New TCP/8080 session | Independent TCP/8082 path |
|---|---|---|---|---|
| ESR-001 | Initial allow policy | Must exchange a fixed positive-control line | Not evaluated | Must connect |
| ESR-002 | Remove the TCP/8080 admission rule but keep generic established acceptance | Must still exchange a fixed line | Must be blocked | Must still connect |
| ESR-003 | Add a source/service-specific deny before generic established acceptance | Must receive no response within the fixed timeout window | Must be blocked | Must still connect |

The second case is not a recommendation to preserve revoked access. It is a control demonstrating that *removing permission to start a session is not necessarily revoking permission for a session already in progress*. The third case expresses explicit revocation semantics for this fixed test tuple.

## Isolation and safety envelope

- Uses two temporary Linux network namespaces linked only by one veth pair.
- Fixed IPv4 addresses `198.18.1.1/30` and `198.18.1.2/30`; target TCP port `8080` plus a separate allowed positive-control service on TCP `8082`.
- No IPv4 or IPv6 default route, bridge, NAT, external route, packet capture, or host-firewall modification.
- Policy updates execute only inside the temporary server namespace.
- No caller-supplied target, interface, port, command, or policy text is accepted.
- Requires both `--confirm-session-revocation-lab` and `LUMINOUS_EDGE_SESSION_LAB_ACK=EPHEMERAL_NAMESPACES_ONLY`.
- Effective rulesets are read back and checked for order: the explicit deny must precede the generic established-state accept, and the independent TCP/8082 allow rule must be present in every stage.
- A fresh TCP/8082 connection is required before and after each policy transition. If this positive control fails, the run fails instead of interpreting a broken path as successful denial of TCP/8080.
- The runner kills child processes, removes both namespaces, and verifies that their names are absent. A force-kill or host failure can still interrupt cleanup; use disposable runners.
- The echo payload consists only of fixed synthetic markers inside the isolated namespaces. It is not a production application test.

A timeout is a bounded observation, not proof that a packet was dropped at an exact instant. The summary reports policy-install command duration separately from the client's fixed response-observation window. These values are not a deployment SLA or a network-performance benchmark.

## Proposed performance and correctness scorecard

The first objective is semantic correctness, not speed.

| Metric | Qualification rule | Status |
|---|---|---|
| Positive control before change | Every trial exchanges the pre-change line | Required |
| Existing session after admission removal | Must remain usable under the control policy that retains generic established acceptance | Required control |
| New session after admission removal | Zero unintended successful connections | Required |
| Existing session after explicit revoke | No response within the declared observation window; server stays alive | Required |
| New session after explicit revoke | Zero unintended successful connections | Required |
| Independent allowed path | TCP/8082 remains connectable in every stage, including after revocation of TCP/8080 | Required |
| Rule-order safety | Effective tuple deny precedes broad established acceptance | Required |
| Cleanup | Zero named namespaces remain after successful runs | Required |
| Policy identity | Record SHA-256 of the effective ruleset for each stage | Required integrity check; not authentication |
| Replication | At least 30 randomized paired trials per qualified kernel/ruleset revision; report every failure | Proposed target |
| Revocation delay | Report distribution from policy update to observed application cutoff; use p50/p95/p99 and the declared timeout, not a single mean | Proposed measurement |
| Rollback | Restore the prior approved policy and independently re-probe both allowed and denied paths | Next qualification step |

Do not claim “zero false permits” from a tiny fixed matrix as a universal security result. Report the case denominator, kernel version, nftables version, number of trials, all mismatches, and whether a trial was inconclusive. An inaccessible endpoint, dead listener, or failed positive control invalidates the corresponding denial result.

## Industry comparison and differentiation

- [Batfish differential reachability](https://batfish.readthedocs.io/en/latest/notebooks/differentialQuestions.html) analyzes how successful reachability changes between network snapshots. Luminous should use configuration analysis like this as a pre-deployment layer rather than reimplementing a mature network parser.
- The Linux [nftables conntrack model](https://wiki.nftables.org/wiki-nftables/index.php/Matching_connection_tracking_stateful_metainformation) is the relevant mechanism-level baseline for the semantics exercised here.
- For throughput comparisons, first validate the L2/L3 path and then use a repeatable methodology such as [IETF RFC 6349](https://www.rfc-editor.org/rfc/rfc6349.html), which covers path MTU, baseline RTT, bottleneck bandwidth, socket buffers, throughput, transfer-time ratio, TCP efficiency, and buffer delay. This namespace lab is not such a throughput test.
- Commercial platforms must be evaluated against the exact hardware model, firmware/software release, rule features, licenses, topology, and configuration. No performance or security superiority claim is justified by this test.

The defensible differentiation goal is an **evidence-driven change contract**:

`declare intended semantics → inspect effective policy → apply change → probe the existing flow and a new flow independently → reconcile observations → promote or roll back`

The report must distinguish controller acknowledgements, policy snapshots, and independently observed packet behavior. A policy hash binds bytes; it does not authenticate the runner or the device.

## Remaining gaps

This lab does not yet test physical APs/firewalls, UDP behavior, IPv6 session revocation, DNS/NAT64, multi-device rollout, controller partition, stale controller state, packet-loss conditions, failover, application transaction success, or rollback. Those should be separate registered cases with positive controls. PR #87 adds a challenge-bound freshness layer, but production replay protection still needs a durable, atomic nonce ledger and authenticated evidence. The next policy-lab extension should add a multi-stage rollback matrix before testing any external device.

## Run

On a disposable Linux test host with Rust, `iproute2`, and `nftables` installed, run as a user authorized to create network namespaces and namespace-local nftables rules:

```sh
LUMINOUS_EDGE_SESSION_LAB_ACK=EPHEMERAL_NAMESPACES_ONLY \
  cargo run --manifest-path tools/edge-policy-session-revocation-lab/Cargo.toml -- \
  --confirm-session-revocation-lab
```

The executable has fixed lab-only child modes, and the coordinator rejects arbitrary targets and policy commands. The TCP/8082 service is only a fixed positive control, not a second target under test. Do not run this test on a production host.
