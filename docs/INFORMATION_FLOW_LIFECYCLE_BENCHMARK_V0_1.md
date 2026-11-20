# Agent Information-Flow Lifecycle Benchmark v0.1

**Status:** synthetic contract and fixtures only. This is a benchmark supplement to AUTH-CORE-002 #503 and the existing Agentic Endpoint Assurance Benchmark v1 in PR #141. It does not create a second provenance manifest, memory store, policy engine, DLP service, flow-control runtime, effect observer or CI runner.

## Research basis and competitive framing

HCP v2 (arXiv:2606.29073v2, 7 November 2026) explicitly includes source-and-target data-flow authorization in its execution invariants. Its 132-condition third-party MCP study reports that per-call grants alone did not stop a secret being passed as an ordinary argument to an authorized sink; session source budgets blocked the copy but refused unrelated actions; explicit handle pipes/sealed reads blocked tested handle-mediated exfiltration while preserving unrelated workflows. HCP is a serious comparator, not a mechanism we claim to have invented.

AgentCap (arXiv:2611.00729, 31 October 2026) already applies parameter-level provenance at invocation time, alongside short-lived budget-bounded capabilities and attenuating parent/child delegation. We do not claim parameter provenance, capability attenuation or source/sink controls as novel.

The additional measurement target is lineage after semantic transformation and context changes: extraction, summarization, paraphrasing, embeddings, memory store/retrieval, serialization and child-agent handoff. A new summary or embedding must not become an independent trusted source. A permitted tool call does not imply that its payload is authorized.

A November 18, 2026 systematic review of 126 peer-reviewed agent-workflow papers notes memory poisoning, compounding pipeline errors and a lack of standardized trajectory-level evaluation. The review's screening was performed by one reviewer, so this is a research signal, not an exhaustive census.

## Paradigm: actions are not the whole security boundary

Permission to call \`invoice.pay\`, \`records.append\` or \`export.send\` does not establish that every parameter or data item sent through that operation is legitimate. This benchmark separates:

1. **Action-selection origin:** who selected the action and whether the task was fully specified, parameter-open or action-open.
2. **Parameter-value origin:** which exact source records contributed to each security-sensitive field.
3. **Transformation lineage:** which extraction, normalization, summary, paraphrase, embedding, memory and handoff operations produced the value.
4. **Sensitivity and integrity:** the strongest source sensitivity and weakest source integrity must survive derivation unless an exact owner-authorized declassification receipt permits a bounded downgrade.
5. **Sink scope:** exact resource owner, tenant, purpose, audience, operation and sensitivity ceiling.
6. **Observed effect:** an independent sink-side observer must confirm the actual payload bindings; an agent report is not effect truth.

The canonical Origin/Intent Provenance Manifest and task-precision semantics remain owned by AUTH-CORE-002 #503. This benchmark consumes that contract; its fixture fields are not a replacement manifest.

## Semantic model

- A source node refers to a source record with source class, integrity, sensitivity, tenant, purpose, content digest and receipt horizon.
- A derived node names parent node IDs and a transformation kind. Its actual origin is reconstructed as the union of all parent source IDs.
- A derived node's declared source set must equal that reconstructed union. Its sensitivity cannot silently fall below the maximum parent sensitivity; its integrity cannot become more trusted than the least-trusted parent.
- Unknown/expired source receipts and cyclic/missing lineage remain indeterminate. Explicit source revocation denies reuse from that source at later flow steps; a later revocation does not retroactively rewrite an earlier valid effect. Explicitly revoked sources are denied at the event-time boundary; a later revocation does not retroactively rewrite a prior valid effect. They are never silently promoted to trusted.
- A declassification receipt is valid only for the exact source set, output node, sink, audience, purpose, owner and step horizon. A valid receipt is not general authorization for other sinks or future outputs.
- A parameter-open task authorizes only the fields and source IDs explicitly delegated. An allowlisted source that can populate an amount cannot also change a recipient or action.
- An action-open task must constrain action selection to a finite declared action set and trusted action origin.
- The independently observed parameter digests must match the exact node bindings for the sink event.

The integrity lattice in v0.1 is a simplified test vocabulary, not a universal information-flow label standard. The benchmark does not recompute canonical content hashes, verify signatures, or prove cryptographic receipt authenticity.

## Frozen synthetic corpus

20 cases: 5 positive controls, 13 negative controls and 2 indeterminate controls. They cover fully specified parameters, named-source parameter delegation, recipient substitution, source-driven action changes, finite action-open choices, action-origin substitution, summary/paraphrase laundering, embedding and memory lineage, child-agent handoff, mixed-origin joins, secret-to-public declassification, stale/misbound receipts, cross-tenant memory, unknown source receipts, cyclic lineage, explicit memory-source revocation at and before the flow step, benign named-source utility, blocked exfiltration, a deliberately unsafe allow, and independent sink-payload substitution.

Files:
- \`benchmarks/agentic-endpoint-security-v1/information-flow-lifecycle.schema.json\`
- \`benchmarks/agentic-endpoint-security-v1/information-flow-lifecycle.fixtures.json\`
- \`tools/agentic-endpoint-security-v1/check_information_flow_lifecycle.py\`
- \`tools/agentic-endpoint-security-v1/test_information_flow_lifecycle.py\`

The standard-library checker labels schema validation as \`STRUCTURE_CHECKED\`; the full schema is Draft 2020-12 for external validators. The fixture corpus status is \`synthetic_fixture_suite_not_executed\`. A checker pass is not an agent runtime test.

## Independent metrics (no aggregate score)

- action-origin decision and action-origin violations;
- flow decision and system-admission mismatch;
- lineage erasure, unknown lineage and cyclic lineage counts;
- cross-tenant and cross-purpose flow attempts;
- untrusted source contributions to guarded parameters;
- source/field rule failures and declassification receipt failures;
- independently observed effects and payload binding mismatches;
- unauthorized disclosures, unsafe admissions and false refusals;
- sensitivity/integrity propagation by transformation depth;
- p50/p95/p99 lineage-check latency, model/tool tokens, memory and storage overhead when a live adapter is available.

Hard targets for a live matched campaign: zero unauthorized protected disclosures; zero source-origin laundering through transformations, memory or handoffs; zero cross-tenant/cross-purpose retrieval; zero acceptance of stale/misbound declassification; 100% acceptance of eligible positive controls; unknown lineage is hold/indeterminate, not a clean pass.

## Comparator protocol

Use HCP v2 source-to-target/handle-pipe controls and AgentCap's parameter-provenance enforcement as the first external baselines. Reproduce their source/runtime or mark figures as literature-only. Match model, task, parameters, source corpus, credentials, sink state, observer, attack budget and false-refusal denominator. A blanket "deny all external data" control is not a winning baseline: parameter-open named-source extraction and unrelated benign operations must continue to work.

NIST's 27 October 2026 agent-identity tenets support task-scoped ephemeral authority, policy/reasoning separation, delegated accountability and independent evidence, but this benchmark is not a NIST profile or conformance claim.

## Claim ceiling

v0.1 tests synthetic lineage semantics. It does not implement AUTH-CORE-002, authenticate real connector receipts, inspect a live vector database, instrument an LLM's internal representations, prove complete memory lineage, verify cryptographic declassification, prevent runtime disclosure, or qualify production privacy. Live promotion requires real source-bound receipts, instrumented transformations and memory, independently captured sink payloads, exact-head CI, matched HCP/AgentCap baselines and independent replication.
