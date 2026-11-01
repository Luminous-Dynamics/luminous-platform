# Event-level degraded request-time impact v0.1

## Why this is separate from request-count impact

PR #192 measures exact unique affected-request counts and impact rates. It does not estimate time. A shortcut such as \`unique_affected_requests × full_window_duration\` is invalid because a request may be degraded for only a fraction of the window.

This module adds a separate duration estimand based on observed event intervals. It is additive and does not change the #192 count-based estimand.

## Estimands

For each keyed request pseudonym \(r\), merge all failed, slow and business-incorrect intervals into one per-request union \(I_r\), including overlaps and adjacency across categories. Let \(W\) be the preregistered half-open observation interval.

\[
D_r = \mu(I_r \cap W), \qquad
D_{\mathrm{total}} = \sum_r D_r
\]

The evaluator reports:
- unique failed, slow and business-incorrect request counts;
- unique union of affected requests;
- observed degraded request-milliseconds inside \(W\);
- longest continuous degraded episode for any single request;
- number of requests still degraded at the right observation boundary;
- number of interval rows merged due to overlap or adjacency;
- Candidate-minus-NoAction deltas for both request count and observed request-ms.

Overlapping categories never triple-count the same request-millisecond. Separate degraded episodes for one request retain healthy gaps.

## Interval and clock semantics

All intervals use half-open \`[start_ms, end_ms)\` semantics. Starts must be observed inside the declared cell. Explicit recovery must be after start and no later than cell end. A missing recovery is right-censored and clipped at the cell end. The observed part is counted, while the right-censored request count is reported independently. This says nothing about how long degradation continued after the cell closed.

An interval whose start precedes the cell is left-censored and rejected for the exact-duration estimand; it must not be guessed from request counters. A future extension may add a trusted boundary-state observation that resolves this.

Each cell pins clock source, clock epoch, monotonic-to-UTC mapping digest, timestamp resolution, and maximum clock uncertainty. The observed uncertainty must not exceed the contract. The offline evaluator checks binding and thresholds, not the truth of the clock assertion; the interval bytes and clock evidence must be authenticated by the typed source-evidence gate (#191 / issue #190).

## Pairing

The interval evaluator reuses #192's exact pair contract. Candidate and NoAction cells must use the same scenario/cluster, service, request class, phase, workload and schedule digests, exact interval, demand contract, key epoch, observer identity/control domain and executor separation. Both arms must deliver the same actual offered-request count within the pinned demand band.

The aggregate paired deltas are emitted only when every expected pair contains exactly one valid cell per arm and there are no unmatched observations. A missing, duplicate, mismatched, stale, future, incomplete, truncated or clock-invalid cell is indeterminate.

## Adversarial validation

The Rust suite includes cross-category overlap, adjacent intervals, disjoint episodes, right-censoring, explicit recovery exactly at the boundary, signed pair deltas, and an independent per-millisecond occupancy oracle. It also includes 64 fail-closed mutation vectors for request tokens, event IDs, interval bounds, clock identity/uncertainty, pair contract, source identity, receipt freshness, and offered demand.

These are evaluator conformance tests, not measured SRE-agent results.

## Claim ceiling

The arithmetic is exact for the supplied authenticated event intervals. It does not prove the ingress observer saw every request, that the category labels are semantically correct, that the clock mapping is accurate, that source publishers are independent, or that the benchmark is causally representative. It also does not estimate full request lifetime beyond the observation boundary. Those require source authentication, independent ingress completeness, validated business oracles, defensible candidate/no-action assignment, and cluster-aware inference.

No production/customer traffic is used. Platform Saga remains the episode coordinator; participant-owned ledgers own external effects; independent observers own service truth; Nixward remains the only host transaction/interlock/journal/recovery authority.
