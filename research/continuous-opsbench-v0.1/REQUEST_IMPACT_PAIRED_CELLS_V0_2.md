# Exact request-impact unions and paired counterfactual cells v0.2

Research-only extension for issue #192. It is deliberately additive: the v0.1 continuous lifecycle evaluator remains intact, but its \`max(failed, slow, incorrect)\` estimate remains a lower bound and must not be interpreted as exact request impact.

## Measurement shift: counters → request-set evidence

A request is degraded when it violates at least one preregistered service or business condition. The unique-impact numerator is the set union:

\[
D = F \cup L \cup B,\qquad |D| = \text{unique degraded requests}
\]

where \(F\), \(L\), and \(B\) are respectively failed, slow, and business-incorrect request sets. The dimensions remain separately reportable. The union counts an affected request once even when it violates multiple conditions.

This distinguishes the two important controls:

- Disjoint: \(|F|=|L|=|B|=10\), with no overlap, gives \(|D|=30\).
- Fully overlapping: \(|F|=|L|=|B|=10\), with all three sets equal, gives \(|D|=10\).

Using \`max()\` returns 10 for both and therefore cannot identify exact harm. Summing the counters returns 30 for both and also fails. The evaluator uses keyed request pseudonyms and explicit set membership, never raw request IDs.

## Exact pairing contract

Before a run, the evaluator receives one immutable contract per expected pair. It pins pair and scenario cluster, service, request class, phase, exact interval, workload and phase-schedule digests, offered-demand contract, pseudonym-key epoch, expected observer identity/control domain, executor control domain, receipt freshness, and event-row ceiling.

Each pair requires exactly one Candidate cell and exactly one NoAction cell. Both must agree with the same contract and each other on the actual offered-request count. Missing, duplicate, extra, substituted, stale, future-dated, truncated, misaligned, or under/over-delivered cells make the pair indeterminate. No aggregate-duration substitution is allowed.

The offered-demand contract has an explicit minimum and maximum. In this first deterministic evaluator, candidate and no-action actual offered counts must also be identical within that range. This is intentionally stricter than comparing normalized totals across unequal demand.

## Metrics

For each valid cell:

- Unique failed requests, unique slow requests, and unique business-incorrect requests.
- Unique union cardinality.
- Duplicate event rows (deduplicated, but visible as an evidence-quality counter).
- Unique degraded request-time: \(|D| \times \text{cell duration}\).
- Demand-weighted impact rate: \(|D| / \text{offered requests}\).

For each exact pair:

\[
\Delta_{pair} =
  \text{Candidate degraded request-time}
  - \text{NoAction degraded request-time}
\]

Positive values mean the candidate caused more measured harm; negative values mean less. The pooled delta is emitted only if every preregistered pair is valid and there are no unmatched observations. Per-pair signs and distributions are retained. No single compensatory recovery score is introduced.

## Privacy and trust boundary

The event token must be a 32-byte keyed HMAC pseudonym encoded as 64 hexadecimal characters. Plain hashes of low-entropy request IDs are not privacy protection. The key never enters the evaluator; its epoch is pinned in the pair contract. Key rotation is a new epoch and cannot be silently mixed within a pair.

The offline evaluator checks source identifiers, control-domain separation, key epoch, binding, and freshness. It does **not** verify signatures or prove that an observer is independent. Production ingestion must bind the observation bytes to the existing typed source-evidence gate (#191 / issue #190), a durable replay guard, and a verifier-owned observer policy. Do not add a caller-supplied \`signature_verified: true\` flag.

## Calibration and benchmark protocol

1. Generate synthetic request cohorts with known failed/slow/incorrect memberships.
2. Include disjoint, full-overlap, partial-overlap, and adversarial duplicate-row controls.
3. Compare evaluator union cardinality against an independently implemented test oracle; do not reuse evaluator helpers in the oracle.
4. Exercise key-epoch changes, token truncation, source substitutions, stale/future receipts, exact interval mismatches, missing/extra/duplicate cells, and candidate-induced demand under-delivery.
5. Keep all repeats nested under scenario/fault-schedule/topology cluster IDs. Pair at the cell level; resample independent clusters, not individual requests.
6. Freeze the primary estimand, class weights, comparison plan and exclusions before confirmatory runs.

The included suite contains two explicit overlap controls, exact-pair controls, and a 48-case adversarial mutation matrix. These are evaluator conformance tests, not empirical benchmark results.

## Claim ceiling

This establishes exact set-union arithmetic only for the supplied event-token records and the declared observation cells. It does not establish that the observer actually received every offered request, that the publisher is authentic, that the observer is independent in deployment, or that the scenario distribution represents production. Missing or unauthenticated evidence is indeterminate, never zero. Production causal claims still require randomized or otherwise defensible paired runs, independent ingress truth, source authentication, cluster-level uncertainty, and an explicit workload-representativeness argument.
