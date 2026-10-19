# Luminous Edge dual-stack ephemeral network-namespace lab v1

**Evidence level:** virtual Linux-kernel network-namespace integration test. Not a physical AP/firewall test, vendor certification, or authenticated evidence.

## Scope and safety envelope

This fixed test covers four TCP probes over an ephemeral veth pair between two temporary Linux network namespaces:

| Case | Address family | Destination | Expected |
|---|---|---|---|
| EPC-901 | IPv4 | `198.18.0.2:8080` | Permitted |
| EPC-902 | IPv4 | `198.18.0.2:8081` | Blocked |
| EPC-903 | IPv6 | `[2001:db8:1::2]:8080` | Permitted |
| EPC-904 | IPv6 | `[2001:db8:1::2]:8081` | Blocked |

The client uses `198.18.0.1/30` and `2001:db8:1::1/64`; the server uses `198.18.0.2/30` and `2001:db8:1::2/64`. Both address ranges are fixed in source. Listeners bind to explicit addresses, so IPv4 and IPv6 sockets are unambiguous.

The server namespace receives a single fixed `inet` nftables table with a default-drop input chain, loopback allowance, a narrow allowance for IPv6 Neighbor Solicitation/Advertisement (required to resolve the direct IPv6 peer at layer 2), and TCP/8080 allowance. TCP/8081 is intentionally not allowed in either address family. Before probing, the runner checks the effective namespace ruleset, requires loopback, Neighbor Discovery, and the TCP/8080 rules, rejects a TCP allow on 8081, and confirms neither namespace has an IPv4 or IPv6 default route. No router/default route is configured and the lab emits no router advertisements.

The runner requires both `--confirm-ephemeral-netns-only` and `LUMINOUS_EDGE_NETNS_ACK=EPHEMERAL_NAMESPACES_ONLY`. It accepts no arbitrary target, port, command, interface, firewall rule or duration from the caller. It does not change the host firewall, create a bridge/NAT/external route, capture packets, or send application payloads; it only opens fixed TCP connect probes. All nftables operations execute inside the new server namespace. CI asserts that the temporary named namespaces are removed after the run. A force-killed process or host failure can still interrupt cleanup, so use a disposable lab/runner.

## Run

On a disposable Linux lab host with `iproute2`, `nftables`, and OpenBSD `nc` installed:

```sh
LUMINOUS_EDGE_NETNS_ACK=EPHEMERAL_NAMESPACES_ONLY \\
  cargo run --manifest-path tools/edge-policy-netns-lab/Cargo.toml -- \\
  --confirm-ephemeral-netns-only
```

The command needs permission to create namespaces and namespace-local nftables state. It emits a JSON envelope with the receipt, redacted probe records and reconciliation report. It does not export MAC/IP identities, interface names, or raw packet content.

## Receipt and trust boundaries

- Corpus digest binds the exact focused dual-stack corpus bytes.
- Approved-policy digest binds the exact fixed nftables script. Effective-policy digest is computed from `nft list ruleset` in the server namespace; effective configuration binds route/address snapshots and the policy digest.
- Each probe record binds case ID, address family, fixed destination port, TCP outcome, exit code, duration, monotonic/wall-anchored time, and effective-policy digest; its record digest is copied into the receipt.
- The result uses evidence class `namespace_lab_observation` with claim ceiling `isolated_linux_namespace_observation_not_physical_or_authenticated`. A SHA-256 digest detects accidental content mismatch; it does not authenticate the runner or device.
- Any unexpected allow/block, missing case, missing tool, insufficient privilege, unexpected default route, startup error or digest/schema inconsistency causes a non-success result. The workflow includes a cleanup gate.

## What the experiment proves—and doesn't

A passing run shows that the declared IPv4 and IPv6 allow/deny controls behaved as expected through one Linux network-namespace + veth + nftables path on the tested runner. It is stronger than a synthetic-only fixture and detects a basic IPv4/IPv6 divergence. The IPv6 positive control only works because the default-drop policy narrowly permits Neighbor Solicitation/Advertisement; dropping those messages would prevent neighbor resolution before the TCP/8080 rule could be tested.

It does **not** test DNS resolver behavior, NAT64, IPv6 extension headers, existing-session revocation, controller loss, RF airtime/interference, roaming, hardware acceleration, physical vendor firewalls/APs, production traffic, attacker detection, or hardware qualification. Four passing cases are not a security certification and do not establish that Luminous beats Meraki or Palo Alto.

The next stage is to expand the controlled lab to alternate-path/session/failover cases and then use the separate AP measurement protocol for authorized physical hardware testing. Keep virtual lab, physical measurements, field evidence and authenticated device evidence as separate claim levels.

