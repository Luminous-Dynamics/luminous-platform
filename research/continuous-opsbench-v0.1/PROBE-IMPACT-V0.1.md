# Probe Interference Evaluator v0.1

Status: synthetic research evaluator. No live probe execution or production impact claim is made.

Related work: issue #257; active value-of-information #185; closed-loop plant #237 / PR #243; demand fidelity #184; exact request-impact accounting #192; experiment preregistration #218/#219.

## 1. Why this is separate from value of information

Issue #185 measures the value, latency, privacy cost and decision risk of evidence acquisition. This evaluator measures an additional quantity: **the operational externality caused by acquiring the evidence**.

A diagnostic operation may be read-only at the permission layer while still consuming CPU, memory, I/O, network, query capacity or rate-limit budget. Such a probe can worsen service latency or business correctness and may even distort the observation it returns.

The v0.1 module is a sidecar. It does not calculate information gain, schedule probes, authorize access, or change native benchmark task success.

## 2. Paired experiment contract

The evaluator compares one probe-enabled run with one matched no-probe run. Both windows must match exactly on:
- scenario and preregistered paired-block identity;
- service and workload class;
- start/end interval;
- offered request count;
- workload-profile, environment and fault-schedule digests;
- independent observer control domain.

The candidate and control may have distinct run and arm IDs, but they must be part of the same preregistered pair. These paired measurements do not, by themselves, establish causal attribution: the experiment must establish randomization/counterbalancing, seed lineage and dependence units as required by #218/#219.

## 3. Independent probe-effect receipt

Every exact probe execution must have one complete, independently sourced impact receipt. The receipt binds execution ID, probe ID, tenant, target, operation digest, run/scenario/arm/pair, observer identity/control domain/adapter digest, measurement interval, evidence digest and resource use.

The resource-impact observer and service-outcome observer are separate trust domains, and both differ from the probe executor. Their adapter digests are frozen in the contract. The impact observer also supplies a contiguous source sequence; missing, duplicated, gapped, stale, incomplete or substituted receipts are indeterminate. Absence of an impact receipt is never interpreted as zero impact.

The resource sidecar totals CPU millicore-ms, memory byte-ms, disk reads/writes, network bytes, query rows and duration. A declared read-only capability is checked separately from measured resource impact; read-only permission does not imply zero impact.

## 4. Service externality

For each service/workload class, the evaluator reports separate paired deltas in:
- failed-request rate;
- slow-request rate;
- incorrect business-result rate.

Each dimension has its own non-compensatory maximum degradation threshold. It does not use a lossy average or allow a bulk-workload improvement to offset a critical-class regression. Resource use is checked against a total probe budget for the paired arm.

Disposition:
- `Pass`: complete, matched evidence; all resource and per-class service deltas within limits.
- `Fail`: mutating probe, probe duration/resource envelope violation, or per-class service delta above a frozen limit.
- `Indeterminate`: missing or invalid execution/receipt/window pairing, incomplete observer coverage, or unknown demand/fault/environment identity.

## 5. Metrics and interpretation

The output is a probe-interference sidecar. Combine it with #185's evaluator-owned decision-risk reduction to produce the research metric **decision-risk reduction per measured operational externality**. Do not let this ratio hide a hard service-safety violation, and do not pool the sidecar into SREGym/EnterpriseOps native scores.

For synthetic confirmatory work, use the same no-action and runbook twins, identical demand/fault schedules, and paired scenario/seed clusters. Report per class/service and all failed, timeout, unknown and incomplete attempts.

## 6. Claim ceiling

The evaluator measures the paired impact of declared probes on supplied synthetic or disposable service observations. It does not establish that all real probe side effects are observable, that the control twin is counterfactually equivalent without a randomized design, or that synthetic impact generalizes to production. Real adapters need provider-specific scope controls, independent resource and service observers, and their own evidence-integrity tests.
