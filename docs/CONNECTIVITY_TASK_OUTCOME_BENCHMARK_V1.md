# Task-first connectivity outcome benchmark v1

Status: synthetic contract prototype. The checked-in fixture is deliberately synthetic; it is not a field observation.

## Paradigm shift

Raw megabits per second do not tell us whether a user completed a task that mattered. A network can move many bytes while essential messaging fails, spend excessive energy per completion, or shift all costs to unpaid volunteers. This contract treats a task attempt—not a byte—as the primary outcome and retains failure/inconclusive denominators.

The run pairs a baseline and a candidate on the same scenario ID, priority, and outage mode. It binds the run to the exact benchmark-profile bytes with SHA-256. The offline validator computes reports from raw trials instead of trusting prefilled summary counts or rates.

## Recorded per trial

- Unique trial ID and pair/scenario identifiers.
- Baseline/candidate arm, task priority, mode (online, WAN partition, route recovery).
- Outcome: succeeded, failed, or inconclusive.
- Monotonic elapsed milliseconds when measured; null is allowed only for an inconclusive trial.
- Bytes sent/received, optionally measured energy in millijoules, and user charge in integer minor units.
- One three-letter currency per run. No exchange-rate conversion or cross-currency arithmetic occurs here.

## Report semantics

For each arm, preserve the attempted denominator and the raw succeeded/failed/inconclusive counts. Success rate is an integer numerator scaled by 1,000,000 and denominator; do not treat an absent denominator as 0%. Latency percentiles use successful tasks only and nearest-rank quantiles, globally and stratified by priority and outage mode. Bytes per successful task, energy per successful task, and charge per successful task are null if there are no successful tasks; energy/charge ratios are also null when any successful task lacks that measurement. Resource totals and observation counts are retained. Integer per-task averages are rounded upward so they cannot understate the reported byte, energy or cash burden. Failed and inconclusive attempts remain in the success-rate denominator.

The validator checks that each pair has exactly one baseline and one candidate with identical scenario, priority, and mode; that trial IDs are unique; that all required priority/mode classes occur; that the profile digest matches its exact file bytes; and that raw counts are not caller-supplied. It reports the four matched-pair cells: both succeeded, candidate-only success, baseline-only success, and neither achieved a confirmed success. The paired success delta is (candidate-only wins − baseline-only wins) / all matched pairs. For this paired binary view, failure and inconclusive both count as "success not confirmed"; their raw counts remain separate in each arm's primary report. No inferential p-value is produced by v1. The report is reproducible from the raw trial list.

## Evidence classes

- synthetic_simulation: schema/policy validation and artificial trials only.
- bench_measured: additionally requires gateway model, hardware revision, firmware image SHA-256, measurement method, and a pseudonymous power-meter token.
- field_observed: additionally requires a pseudonymous site token and a consent-record digest.

These gates are metadata requirements, not authentication. A self-supplied token or declared evidence class does not independently prove a test happened. Human review, protected raw measurements and external authorization remain necessary.

The profile stage is also enforced. The checked-in profile is a three-pair contract-smoke fixture and accepts synthetic evidence only. Lab screening requires at least 30 paired trials in every required priority × outage-mode cell and at least 270 matched pairs total for the 3 × 3 matrix. Field-pilot profiles require at least 100 matched pairs in every cell (900 total); reliability-qualification profiles require at least 300 per cell (2,700 total). Profiles cannot claim a later stage while retaining smoke-test sample sizes.

## Privacy and trust boundaries

The structured schema rejects unknown fields, preventing arbitrary new person-specific, credentials, address or raw-MAC fields from being introduced without a contract change. Tokens must still be genuinely opaque/pseudonymous; syntax validation is not re-identification-risk analysis. Do not publish precise timestamps, location traces, stable device identifiers or trial data that can identify households. Minimize retention and aggregate before sharing. The validator does not authenticate the reporter or prove task success.

## Benchmarks and advancement gates

1. Contract CI: every schema and policy check passes, exact profile byte digest matches, and every reported numerator/denominator is derived from raw trials.
2. Negative controls: duplicate trial ID, malformed digest, wrong profile digest, unpaired scenario, cross-pair metadata mismatch, missing measured-device metadata, and field_observed without consent/site metadata are rejected.
3. Candidate-versus-baseline comparison: the lab-screening profile requires 30 paired trials in each required priority × mode cell (270 pairs for all 3 × 3 cells); a field-pilot profile requires 100 pairs per cell. Publish exact denominator and inconclusives, not just a percentage.
4. Reliability: target at least 99% successful eligible essential-task completion in the defined scenario and zero expired forwards under outage/recovery. For zero observed failures, about 299 independent candidate attempts are needed for a one-sided 95% upper confidence bound on failure probability below 1% (the rule-of-three approximation); clustering by day/site/device reduces the effective sample size. The reliability profile therefore requires 300 paired runs per cell. State confidence intervals, workload and independence assumptions; do not claim a global service guarantee.
5. Latency: report p50/p95/p99 per priority and mode; provisional target is p95 under 1 second for the defined small essential message test under a non-congested local-link profile. WAN/satellite latency belongs in separately defined profiles.
6. Resource economy: target at least 20% lower measured energy per successful task than baseline without lowering task success; separately report bytes per success and cash user charge per success. Targets are hypotheses until physical meters and repeated trials support them.
7. Equity/fairness: report success rates by priority, peer cohort and mode; pair this with separate fair-share allocation tests and actual AP airtime measurements. An aggregate high success rate cannot conceal one group receiving zero service.

Never collapse all metrics into one score without publishing metric weights, confidence bounds and hard safety exclusions. Do not trade away essential-task reliability for a better Mbps, price, energy or fairness score.

## Limits

This package measures internal consistency of benchmark records, not end-to-end task truth, Wi-Fi/roaming compatibility, device certification, cryptographic authenticity, physical network behavior, legal compliance or deployed savings. Store-and-forward semantics follow the challenged-network model described by RFC 9171; security mechanisms are a distinct layer covered by RFC 9172. Network energy reporting should be comparable to measured scope and method; ITU-T L.1331's operational-network energy-efficiency measurement guidance and L.1333's network carbon-intensity KPI provide context, but traffic-based metrics do not replace completed-task outcomes.
