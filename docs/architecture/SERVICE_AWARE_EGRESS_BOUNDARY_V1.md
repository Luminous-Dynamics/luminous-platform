# Service-aware egress boundary v1

**Status:** architecture contract and decision-model prototype. The service egress decision core is not yet a proxy, credential broker, or network isolation mechanism.

## Architectural principle

An agent should not have ambient network authority. It should propose a broker-defined operation such as "read repository file" or "query issue metadata." A trusted supervisor resolves that operation to a fixed endpoint, HTTP method, route template, TLS identity, response limits, and credential handle. The agent receives only the result, not the underlying credential.

The authorization core answers whether one attempt is allowed. A separate broker and OS boundary must enforce that decision and prove the resulting external effect. The workload must not have an alternate direct socket path.

## Required trust boundaries

1. **Policy authority:** trusted policy loader authenticates the policy, checks revocation, persists the rollback floor, and supplies the expected digest/epoch.
2. **Agent boundary:** the agent emits a typed operation request, not arbitrary URL/header/body material with an implicit right to send it.
3. **Decision core:** the Rust crate binds target, subject image digest, broker service ID, operation ID and credential handle into one capability decision.
4. **Broker enforcement:** only the broker has network access and resolves operation IDs into fixed method/route/destination/TLS contracts. It rejects an unregistered operation, a credential mismatch, and any redirect hop that has not been re-authorized.
5. **Credential isolation:** real secrets remain in the broker or external credential service. A workload sees an opaque handle and data returned by approved operations, not a raw reusable token.
6. **Independent observation:** packet capture outside the workload, server-side request logs, credential-sink canaries and durable-state snapshots independently establish what actually happened.
7. **Evidence/provenance:** receipts bind source, policy digest, exact request ID, broker decision and externally observed outcome. Signed provenance binds who produced the receipt; it does not prove the sensor or host was uncompromised.

## Threats the eventual runtime must test

- direct TCP/UDP or raw socket access that bypasses the broker;
- DNS rebinding, changed DNS answers between policy and connect, IP literals, IPv4-mapped IPv6 and alternate address forms;
- redirect chains that cross service, host, scheme or credential scope;
- proxy bypass through alternate environment variables, Unix sockets, inherited descriptors, subprocesses or service-specific clients;
- confused-deputy requests where a read capability is used for a write-like method;
- credential exposure in environment, argv, files, logs, subprocess inheritance, error messages or responses;
- replayed requests, expired receipts, policy rollback and revocation races;
- observation tampering, missing packet capture, inconsistent server logs and falsely labelled successful receipts.

## Model boundary

The current Rust core accepts canonical service and operation identifiers, not arbitrary URLs or HTTP paths. The broker registry must map those IDs to exact transport semantics. The core does not resolve DNS, parse HTTP, verify TLS, open sockets, authenticate policy signatures, or independently prove its adapter supplied truthful verified facts. The word "Verified" means a trusted adapter assertion until that adapter has its own independent tests.

Each redirect hop requires a new request identity and a fresh call to the decision core. The core has no persistent replay ledger; the eventual broker must implement nonce/replay protection and must not reuse an old receipt for a new hop.

## Frozen benchmark gates

The associated machine-readable contract separates decision-model cases from runtime cases. A model unit test cannot pass a runtime-enforcement case. Runtime cases remain not implemented until a supervised broker, direct-egress denial, independent oracle and positive control exist.

Before a SOTA comparison, run the same attack corpus and benign service-operation workload against the unsandboxed positive control, a Linux-native sandbox/proxy baseline, available agent runtimes, and the Luminous candidate. Capture all external effects with one independent oracle and publish raw data, environment digests, failures and confidence intervals.

## Release gate

Do not make a network-confinement or credential-protection claim until the actual launcher prevents direct egress, the broker mediates every route and redirect, secrets remain outside the workload, and an external oracle independently verifies both allowed and denied effects.
