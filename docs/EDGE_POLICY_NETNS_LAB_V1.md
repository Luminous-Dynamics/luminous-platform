# Luminous Edge ephemeral network-namespace lab v1

**Evidence level:** a virtual Linux-kernel network-namespace integration test. Not a physical AP/firewall test, not vendor certification, and not authenticated evidence.

## Safety envelope

This fixed smoke test exercises exactly two TCP ports over an ephemeral veth pair between two newly created Linux network namespaces:

- Client endpoint: \`198.18.0.1/30\`
- Server endpoint: \`198.18.0.2/30\`
- Positive control: TCP/8080 is explicitly permitted and must connect.
- Negative control: TCP/8081 has a listening process but is not permitted; the connection must be blocked.

The runner has no arguments for target IPs, interfaces, ports, commands, rules, or durations. It invokes fixed commands with argument arrays; it does not execute a shell string. It requires both the explicit \`--confirm-ephemeral-netns-only\` argument and \`LUMINOUS_EDGE_NETNS_ACK=EPHEMERAL_NAMESPACES_ONLY\`.

All nftables commands execute *inside the server namespace*. The harness installs no host firewall rules, creates no bridge/NAT/default route, captures no packets or payloads, and sends no application payload—only TCP connect probes. It checks both namespace route tables and refuses any default route. Veth endpoints are moved into the namespaces. Cleanup kills test listeners and deletes the namespaces on normal/error returns; process-kill or host failure can interrupt cleanup, which is why this is intended for disposable CI/lab hosts.

## Run

On a disposable Linux lab host with \`iproute2\`, \`nftables\`, and OpenBSD \`nc\` installed:

\`\`\`sh
LUMINOUS_EDGE_NETNS_ACK=EPHEMERAL_NAMESPACES_ONLY \\
  cargo run --manifest-path tools/edge-policy-netns-lab/Cargo.toml -- \\
  --confirm-ephemeral-netns-only
\`\`\`

The process needs the permissions required to create network namespaces and nftables state within them. It emits a JSON envelope containing the observation receipt, redacted probe records, and offline reconciler report. The envelope does not include MAC/IP addresses, interface names, or raw packet content.

## Receipt semantics

- The corpus digest binds the exact focused smoke-corpus bytes.
- The approved-policy digest binds the exact fixed nftables script; the effective-policy digest is measured from \`nft list ruleset\` inside the server namespace.
- The effective-configuration digest binds the route/address snapshots and effective-policy digest.
- Each observation record includes the case ID, expected path's fixed port, TCP outcome, exit code, timing, and effective-policy digest. Its digest is included in the receipt; the hash protects those record bytes from accidental mismatch but does not authenticate the runner.
- The reported evidence class is \`namespace_lab_observation\`, with claim ceiling \`isolated_linux_namespace_observation_not_physical_or_authenticated\`.
- If port 8080 fails to connect or port 8081 connects, the reconciler reports a mismatch and the CLI exits non-zero. Missing tools, insufficient privileges, unexpected default routes, listener startup errors or schema/digest failures also fail closed.

## What this establishes—and what it does not

A successful result shows that one explicit allow and one explicit deny behaved as expected through the Linux kernel's namespace + veth + nftables path on the tested runner. It is more meaningful than a fully synthetic simulator but does not test Wi-Fi airtime, RF interference, roaming, hardware acceleration, vendor AP/firewall behavior, a production network, revocation/failover, or physical safety. It does not authenticate the evidence producer. The independent physical AP testbed work in PR #77 and the read-only collector in PR #79 remain necessary.

The next layer is to extend the frozen corpus and testbed profile to explicit alternate-path and staged-policy cases, then run against authorized physical equipment. Do not infer performance superiority over Meraki or Palo Alto from this two-case virtual control.

