# Edge policy conformance benchmark v1

**Status:** proposed synthetic corpus and verifier. It does not prove that a firewall rule was enforced, a product is secure, or a physical test occurred.

## Paradigm shift: intent is not enforcement

A control plane can accept a policy, a simulator can predict a decision, and a device can report a successful commit while the effective data plane differs. Luminous should make the final question independently testable: **does the observed path behave as the approved policy says it must, under normal operation and injected failure?**

This complements existing capabilities rather than pretending to invent them. Cisco Meraki’s cloud-managed AP platform emphasizes centralized configuration, monitoring and multi-site visibility ([MR family overview](https://meraki.cisco.com/product-collateral/meraki-wireless-cloud-family-datasheet-managed-wireless-access-points/?file=)). Palo Alto Networks documents policy-match tests and rulebase/policy optimization ([Test Security Rules](https://docs.paloaltonetworks.com/network-security/security-policy/administration/security-rules/test-policy-rule-traffic-matches), [Policy Optimizer](https://docs.paloaltonetworks.com/ngfw/administration/app-id/security-policy-rule-optimization)). NIST’s Zero Trust architecture separates policy decision, administration and enforcement responsibilities ([SP 800-207](https://www.nist.gov/publications/zero-trust-architecture)). The testable opportunity is a portable, evidence-bound assurance loop across intended policy, effective configuration, independent data-plane probes, authority state, change history and recovery under faults—not simply another dashboard.

## Corpus and validator

- Schema: schemas/edge-policy-conformance-corpus-v1.schema.json
- Fixture: examples/edge-policy-conformance-corpus-synthetic-v1.json
- Offline Rust validator: tools/edge-policy-conformance-contracts/

The 13 cases cover segmentation deny, positive local-service controls, revoked/expired/unknown authority, controller partition, DNS/IPv6 alternate paths, established sessions after revocation, stale failover policy, partial rollout, tenant isolation, seeded drift and rollback. The corpus is test intent, not output. A future runner must record what the data plane actually did and compare it with expected behavior.

The validator is designed to reject unknown fields, remote schema references, duplicate case IDs, allow outcomes under stale/revoked/unknown authority, deny cases that expect a permitted probe, missing independent-probe evidence and oversized inputs. It does not control devices, generate network traffic, verify cryptographic signatures or authenticate receipts.

## Non-negotiable invariants

1. **Zero unauthorized effects.** One observed high-risk false permit is a hard failure; throughput or availability cannot compensate.
2. **Unknown or stale authority cannot create a high-risk allow.** Explicitly permitted local essential services may continue only while the locally held policy and authority remain valid and the deployment’s declared safety model permits it.
3. **Independent data-plane observations.** A UI status, control-plane acknowledgement, simulator, policy-match prediction or self-hash alone is not proof of enforced behavior.
4. **No identity or payload export.** Public receipts use run-local opaque tokens, policy/configuration digests and aggregate outcomes; raw MACs, credentials, payloads and identity maps remain local.

## Proposed targets (not measured results)

| Dimension | Qualification target |
|---|---|
| Critical false permits | Zero across all registered critical negative cases and the agreed mutation corpus; any occurrence fails the profile |
| Corpus diversity | Start with these 13 frozen cases; grow to at least 10,000 deterministic, seeded policy/fault mutations before broader security claims |
| Revocation | Proposed p95 no more than 5 seconds for revoking a high-risk existing session, with zero new high-risk sessions after authoritative revocation is effective; report full distribution and uncertainty |
| Stale/unknown authority | 100% rejection of high-risk new grants in registered expiry, revocation and partition cases |
| Drift and rollout | 100% detection of seeded effective-configuration mismatch before candidate promotion; zero promotion on mixed or unverified enforcement state |
| Rollback | 100% restoration of the previously approved policy digest and every registered pre-change probe outcome in tested rollback cases |
| Positive controls | All explicitly allowed essential paths remain functional where their policy and authority are valid; a deny-all candidate cannot pass |
| Evidence completeness | 100% of measured trial records bind policy digest, effective configuration digest, test profile, device/software revisions, authority state, trusted time source and independent probe receipt |
| Clean-path cost | No more than a 1 percentage-point task-success regression; separately report p50/p95/p99 data-plane latency, throughput and CPU/memory overhead |
| Scale | Report validation and rollout/verification latency at 100, 1,000, 10,000 and 100,000 rules; no performance winner claim until measured against frozen baselines |

Thresholds are hypotheses and should be preregistered per deployment profile. A five-second revocation target is not a universal safety standard; some deployments require stricter limits. Keep unknown, failed and inconclusive distinct, and never silently remove failed attempts from a denominator.

## Baselines and fair comparison

Use a tiered comparison rather than pretending one competitor represents the whole category:

1. **Open implementation baseline:** Linux nftables/conntrack and an OpenWrt-based gateway, with and without relevant queue-management tooling.
2. **Cloud-managed networking baseline:** a Meraki configuration and device profile matched to the use case.
3. **Security enforcement baseline:** a Palo Alto NGFW/Prisma Access profile with applicable policy and licenses.
4. **Luminous candidate:** the same topology, paths, identities represented only by opaque test tokens, rule intent, traffic mix, failure schedule and measurement boundary.

For each vendor, record exact SKU/model, firmware/software, license tier, features enabled, topology, policy translation and exclusions. Score a product only for capabilities available to the tested SKU/configuration. Do not compare an AP-only device to a licensed NGFW as if they were functionally equivalent. Run randomized paired trials where practical, publish the frozen corpus and raw local evidence digests, include failed/inconclusive cases, and report confidence intervals and run-to-run variance.

An initial lab can use network namespaces, virtual Ethernet, nftables, a controlled resolver and deterministic fault injection to validate corpus execution and policy-state transitions. That is software/lab evidence only. Then test authorized physical devices on an isolated LAN; never inject faults into a public, customer or household production network. Wi-Fi throughput and loaded latency should reuse the separate AP protocol rather than be inferred from this policy suite.

## How Luminous can win

- Make policy intent portable and vendor-neutral, while adapters remain narrow and expose unsupported semantics.
- Bind each proposed change to the exact authorized plan, effective configuration, independent probes, outcome receipt and rollback identity.
- Separate simulation from enforcement and enforcement from proof; each needs its own evidence class.
- Treat failure modes as first-class: controller disconnected, authority expired, policy drift, partial rollout, failover, stale session, IPv6/DNS alternate path and rollback failure.
- Preserve customer sovereignty with local operation, local evidence retention, exportable contracts and no mandatory cloud dependency for local policy enforcement.

The strategic win is not “we have more firewall features” before the product exists. It is measurable, independent assurance that a configured policy remained effective through the failures the test actually exercised.

## Evidence ceiling and next steps

These files provide a schema, synthetic corpus and proposed offline verifier. They do not execute probes or qualify enforcement. The validator, schema checks and tests must pass exact-head CI before being claimed as validated software. Next: add a lab-only runner that admits an explicitly authorized testbed, applies no changes itself, consumes separately captured probe receipts, and fails closed when effective configuration and probe results disagree. Only later should adapters and physical measurements be used to compare hardware products.

