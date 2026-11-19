# DWF-ANN-PRIVATE v0.1 — privacy-preserving approximate nearest-neighbor track

This protocol is separate from **DWF-HDC-EXACT**. It evaluates approximate vector search under declared privacy and verifier properties. It must not be merged into the exact BinaryHV/Hamming leaderboard.

## 1. Unit of comparison

The unit is a result set that:
1. follows the workload's declared nearest-neighbor semantics;
2. meets the published recall/quality threshold against a pinned exact ground-truth search;
3. is accepted by the declared correctness-verification strategy;
4. respects the owner's authorization and data-scope policy;
5. is measured together with client, server, transfer, setup, and verification cost.

A query that returns a high-recall answer but violates its access-pattern, data-egress or authority policy is not counted as an accepted correct result for that policy.

## 2. Two non-interchangeable tracks

### DWF-HDC-EXACT

- Symthaea BinaryHV, 16,384 bits / 2,048 bytes per vector.
- Exact Hamming distance `popcount(q XOR x)`.
- Exact top-k and deterministic lowest-stable-ID tie-break.
- Correctness is exact equality; no Recall@k approximation.
- Compare local/peer Symthaea, stock FAISS IndexBinaryFlat, the separately pinned unmerged FAISS #5701 patch, and only implementations that preserve the exact contract.

### DWF-ANN-PRIVATE

- Floating-point embedding vectors and an explicit metric (cosine, inner product or L2); do not mix metrics within one leaderboard.
- Exact brute-force ground truth is generated once from frozen corpus/query bytes and pinned numeric semantics.
- Report Recall@k and, where appropriate, task-specific duplicate-tolerant recall as an additional metric, never as a replacement for exact-ID recall.
- Privacy protocols are compared only where they support the same workload/query/index semantics.

## 3. Candidate backend families

Record exact code commit, protocol parameters, security claim, trust assumptions and known leakage for every arm.

| Backend | Protected property (declared, not assumed) | Key caveat to test |
|---|---|---|
| Plaintext | No cryptographic privacy | Control arm; requires consent for query/corpus disclosure |
| SAP / DCPE | Perturbed or transformed corpus/search representation | Similarity/geometry leakage; query protection is not the same as corpus secrecy |
| EMVP / encrypted matrix-vector products | Query and corpus hidden under protocol assumptions | Ciphertext memory/bandwidth and client/server cost |
| BNTM | Encrypted scoring plus malicious-server response verification | Verifier cost and exact security relation |
| Tiptoe / PIR-LWE | Hide the selected index cluster/access choice | Throughput, recall loss and scaling |
| TEE | Confidential execution under attested hardware/software state | Side channels, page/access traces, rollback/debug, vendor and firmware trust |
| FHE hierarchical search | Encrypted query scoring with candidate hierarchy | Selected-cluster access patterns, output leakage, storage/enrollment cost |
| ORAM / BiSON-like | Hide data access patterns under the stated ORAM model | Client state, stash/position-map trust, bandwidth and setup |
| Doubly-oblivious / O2N2-like | Hide access patterns without relying on small trusted storage | Circuit ORAM cost, path leakage, fixed-schedule overhead and recall |
| MPC / threshold | Hide inputs from individual participants under collusion threshold | Communication rounds, dropout and collusion assumptions |

The [EPFL unified benchmark](https://github.com/sacs-epfl/secure-vector-search) is a common-harness reference for Plaintext, SAP, EMVP, BNTM and Tiptoe. Do not silently port its reported figures to different hardware or data.

## 4. Public workload families

Choose a workload family before comparing systems. Do not join unrelated corpora into one score.

- MS MARCO passage embeddings (100K and 8.8M scales) for retrieval/RAG.
- SIFT1M / SIFT100M and Deep1B for standard ANN scaling.
- DataComp-1B CLIP embeddings when reproducing the 2026 FHE hierarchical-search study.
- MSONG where the selected backend and data license support it.
- A separate synthetic BinaryHV-derived track only when a protocol can correctly represent bits and exact Hamming semantics.

For each corpus, publish:
- origin, license and preprocessing/encoder commit;
- dimension, precision, normalization, quantization and distance metric;
- exact raw bytes and SHA-256;
- query set and a disjoint train/dev/test split;
- exact ground truth, tie policy and recall calculation;
- duplicate/near-duplicate treatment, if any.

Avoid tuning hyperparameters on the same query set used for final evaluation. Use separate tuning and held-out test queries; report all operating points selected before the final run.

## 5. Required scoreboards

Do not collapse them into a single weighted privacy score.

### Retrieval utility
- Recall@1/10/100, exact-ID recall and duplicate-tolerant recall separately.
- Precision@k, nDCG or task-specific quality where relevant.
- Stability across seeds, query cohorts and corpus scale.

### Privacy properties
Report each observer separately:
- query confidentiality;
- corpus confidentiality;
- access-pattern obliviousness;
- query-output leakage (scores, ranks, IDs, thresholds, number of results);
- repeated-query linkage and search-pattern leakage;
- metadata leakage (timing, sizes, frequency and traffic);
- malicious-server verifiability;
- collusion resistance and trusted-storage assumptions.

Privacy mechanism labels are not proof. Attach the protocol theorem/security model where available and an empirical leakage report for observable channels.

### Cost and efficiency
Measure full lifecycle, not server kernel alone:
- one-time index build/enrollment/encryption and amortized cost;
- client query encoding/encryption and key setup;
- server index load, execution, memory and accelerator occupancy;
- network bytes and round trips per query;
- client decryption, reranking and result selection;
- independent correctness verification;
- total request-to-accepted-result p50/p95/p99 and queries/s;
- CPU/GPU/RAM/VRAM/storage and energy;
- provider-side resource reservation and opportunity cost.

### Adversarial leakage
- embedding inversion / reconstruction where applicable;
- membership and attribute inference;
- candidate identification and rank leakage;
- access-set Jaccard vs query similarity, shared-cell lift;
- co-access graph / neighbor recovery@k and parent/hierarchy ARI;
- repeated-query intersection and query linkage;
- response-profile inference from scores, top-k IDs and result count.

For each attack, declare attacker role, view, side information, linkage ability, training/test split and confidence intervals. Keep offline trace analysis separate from online attack execution.

## 6. End-to-end measurement boundary

For each query, capture:

`client_prepare -> authorize -> encrypt/encode -> request -> server_load -> execute -> response_transfer -> decrypt -> rerank/select -> independent_verify -> owner_accept`

Also capture rejects, deferrals, retries, duplicate attempts, verifier failures and `INDETERMINATE` outcomes. The denominator must include every offered query; do not omit failed or uncertain work.

For pre-encrypted datasets, report one-time enrollment and storage expansion separately. For on-demand encryption, include it in cold query cost. Do not compare cold and warm numbers without clearly labeling setup amortization.

## 7. Current research comparators (2026-11-10 snapshot)

- **Unified privacy vector-search benchmark:** https://arxiv.org/abs/2608.01192 ; code: https://github.com/sacs-epfl/secure-vector-search
- **BiSON:** https://www.microsoft.com/en-us/research/publication/bison-billion-scale-oblivious-nearest-neighborsearch-in-milliseconds/
- **FHE hierarchical billion-scale ANN and access leakage:** https://arxiv.org/abs/2608.21131
- **O2N2 doubly-oblivious ANN:** https://arxiv.org/abs/2611.03415

Reported research results are external reference points, not reproduced Luminous results. BiSON/FHE/O2N2 and the unified benchmark predominantly use floating-point approximate ANN workloads. They do not establish feasibility or performance for exact 16,384-bit BinaryHV/Hamming.

## 8. Initial gates (targets, not achieved)

- 100% of arms bind a common corpus/query/ground-truth manifest, or record an explicit non-comparable deviation.
- 100% of claimed privacy properties identify attacker, view, protected secret, outputs and leakage.
- Zero false accepted results in the frozen correctness controls; independent verification is measured separately.
- Every arm reports end-to-end accepted-result latency and resource cost, not only server time.
- Repeat-query and access-pattern attacks are mandatory where the backend exposes index/cluster selection.
- No "doubly oblivious" label unless both access and data-obliviousness assumptions are actually met by the reviewed protocol.
- No exactness claim on approximate recall-based results.
- All claims carry schema-backed manifests and explicit evidence class.

## 9. Evidence and claim boundary

Use the manifest schema in `benchmark-run.schema.json`. A schema-valid manifest is not proof of protocol security, correct artifact hashes, valid measurements or implementation correctness. Preserve source commit, exact parameters, raw run files, reviewer identity, and claim limits.

This track specifies how to compare systems. It does not implement cryptography, ORAM, a secure index, a scheduler, an attestation service or a correctness oracle.
