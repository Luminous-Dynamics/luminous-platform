# Edge policy observation reconciler v1

**Status:** offline reference evaluator only. It does not open sockets, generate probes, configure devices, authenticate observations, or qualify a network.

## Purpose

The conformance corpus in \`EDGE_POLICY_CONFORMANCE_BENCHMARK_V1.md\` describes the expected outcome of each policy case. This tool makes the next boundary executable: it accepts a submitted observation receipt, checks its schema and exact corpus-byte digest, matches observations to unique case IDs, and reports mismatches, gaps, evidence insufficiency and target violations.

It is deliberately **not** the live lab runner from issue #81. A future probe runner must separately obtain observations on an explicitly authorized isolated testbed. The reconciler must not infer a packet was blocked because a controller says a rule was installed.

## Files

- Receipt contract: \`schemas/edge-policy-observation-receipt-v1.schema.json\`
- Rust crate and CLI: \`tools/edge-policy-observation-reconciler/\`
- Existing expected-case corpus: \`schemas/edge-policy-conformance-corpus-v1.schema.json\` and \`examples/edge-policy-conformance-corpus-synthetic-v1.json\`

Run a fully synthetic smoke test:

\`\`\`sh
cargo run --manifest-path tools/edge-policy-observation-reconciler/Cargo.toml --bin edge-policy-observation-reconcile -- --smoke schemas/edge-policy-conformance-corpus-v1.schema.json examples/edge-policy-conformance-corpus-synthetic-v1.json schemas/edge-policy-observation-receipt-v1.schema.json
\`\`\`

The smoke command creates a receipt in memory from the corpus and evaluates it. It does not write a device observation or send traffic. For a submitted receipt, pass four paths: corpus schema, corpus JSON, receipt schema, receipt JSON.

## Trust and result semantics

- Exact corpus **file bytes** are SHA-256 bound. Reformatting the corpus intentionally changes the digest.
- Unknown observation cases, duplicate case observations, duplicate observation IDs, schema violations, remote schema references and digest mismatch are rejected.
- Missing, inconclusive and not-run cases cannot count as success.
- An expected deny observed as permitted is a mismatch; a critical/high-risk false permit is separately counted and overrides performance success.
- An expected allow observed as blocked is also a mismatch. A deny-all policy cannot pass.
- A control-plane acknowledgement, simulator prediction or policy-match prediction is not an independent data-plane probe.
- Lab-claim receipts must declare a monotonic clock with wall-clock anchor and bind per-observation evidence digests. The validator can check the declarations and digest syntax but **cannot authenticate the clock, device, artifact or origin**.
- A matching lab-claim receipt is reported only as \`reported_observations_consistent_not_authenticated\`; that status is not a qualification, certification or proof of enforcement.
- Synthetic output is always labelled \`synthetic_fixture_consistent_not_qualification\`.
- Response-time target violations are counted separately from security mismatches. Profiles must decide how to treat them for qualification.
- Public receipts use a syntax-restricted opaque target token; the schema cannot prove the value is not re-identifiable. Never include MAC/IP identities, credentials, payloads, site names or identity mapping tables.

## Adversarial tests

The test suite covers exact corpus digest binding, unexpected permit, unexpected block on a positive control, missing and duplicate cases, unknown cases, non-independent evidence, missing lab evidence/time anchors, target violations, and remote schema references. These are software-level contract tests, not packet-level tests.

## Follow-on work

1. Issue #81: create an explicitly authorized, read-only lab probe runner with allow-listed endpoints, protocol and rate/duration bounds.
2. Bind receipts to protected local raw artifacts and exact device, firmware, testbed and effective-policy identities; if signing/attestation is added, define key enrollment and revocation rather than trusting a self-provided signature field.
3. Add repeatable Linux network-namespace/nftables integration experiments with positive and negative controls, then independently verify behavior.
4. Cross-check intended reachability changes with Batfish where supported; use Hubble/Cilium only for Cilium-managed traffic and retain independent probes for the wider network.
5. Physical AP/firewall tests and competitive claims require their own authorized lab, matched baselines and measured trials.

