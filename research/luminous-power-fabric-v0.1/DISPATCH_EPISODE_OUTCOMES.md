# Power Fabric dispatch episode outcomes and paired campaign comparison — v0.1

**Parent:** [Issue #440 — non-compensatory dispatch episode outcomes and paired campaign comparison](https://github.com/Luminous-Dynamics/luminous-platform/issues/440)  
**Builds on:** [dispatch benchmark PR #433](https://github.com/Luminous-Dynamics/luminous-platform/pull/433), [benchmark passport PR #430](https://github.com/Luminous-Dynamics/luminous-platform/pull/430)

**Status:** typed outcome contract, schema and synthetic fixtures only. No policy runner, statistical qualification, physical result or controller superiority is claimed.

## 1. Paradigm shift: no composite score can hide a failed mission

An EMS can reduce fuel and still fail the customer by missing a critical interval, violating a terminal reserve, dispatching beyond a component capability, or acting on stale telemetry. A weighted average invites exactly the wrong trade: enough efficiency improvement can numerically compensate for a safety or service failure.

Power Fabric therefore treats each mission episode as an evidence-bearing result vector. The disposition comes only from mandatory hard gates:

- **FAIL:** at least one hard gate failed.
- **INCONCLUSIVE:** no gate failed, but at least one required gate is unknown.
- **PASS:** every required hard gate passed.

The six mandatory gates in v0.1 are critical service, qualified capability, terminal reserve, telemetry integrity, protection readiness, and required power events. A future product may add gates, but it must not remove or weaken these without a versioned protocol change and explicit hazard review.

## 2. Standards and industry comparison

The test ladder should map to:
- [IEC TS 62898-3-5:2026](https://webstore.iec.ch/en/publication/84175): HIL, commissioning and periodic tests for microgrid monitoring/control and energy-management systems. Its published contents include dispatch optimization, forecasting, demand-side integration, flexible-resource management, anti-maloperation locking/alarm, voltage/frequency regulation, switching, mode transitions, active/reactive-power control, islanding detection and black start.
- [IEEE 2030.8-2018](https://standards.ieee.org/ieee/2030.8/6169/): microgrid-controller test procedures and comparison metrics. [P2030.8](https://standards.ieee.org/ieee/2030.8/12543/) is an active revision project; track its status separately from the published edition.
- [NREL/EPRI controller HIL/PHIL testbed methodology](https://www.nrel.gov/docs/fy20osti/71338.pdf): useful historical comparator for baseline, planned/unplanned separation, islanded operation, protection, dispatch and reconnection test cases.

The 2026 EMS decision-making review identifies persistent comparison gaps: architecture-aware comparisons, islanded/offshore contexts, sufficiently fine online re-optimization, lifecycle-aware battery cost and hardware validation. Power Fabric should make those explicit strata instead of assuming one benchmark topology represents every microgrid. These references inform the protocol; they do not make Power Fabric compliant.

## 3. Episode result identity

Each episode binds:
- scenario ID and exact scenario digest;
- matched-input digest shared by the reference and candidate policies;
- policy ID and exact policy build digest;
- unique run ID;
- evidence class and test environment;
- evidence-bundle digest, raw-trace digest and analysis-build digest;
- data-rights reference and independent-assessment reference where applicable.

Evidence class and test environment are not interchangeable. Synthetic evidence must use a synthetic environment; modelled evidence must use simulation; measured/witnessed/accredited evidence must name a physical test environment (controller-HIL, power-HIL, controlled laboratory, commissioning, periodic, site-operational or field). A HIL record is not automatically a physical site-service result. Mycelix owns cryptographic verification and trust policy; this module checks the declared typed classification and digest shape only.

## 4. Required result vector

### Hard gates
1. Critical service.
2. Qualified component/capability envelope.
3. Terminal reserve obligation.
4. Telemetry integrity and freshness.
5. Protection readiness.
6. Required power-duration events.

### Service and energy reconciliation
Record required critical service energy, delivered critical service, unserved critical energy and the maximum allowed unserved amount. Require:

`required = delivered + unserved ± declared tolerance`

If the critical-service gate is PASS, unserved energy cannot exceed its declared allowance.

### Reserve
Record the signed terminal reserve margin relative to the mission obligation. A negative value cannot coexist with a PASS reserve gate.

### Resource channels (kept separate)
- fuel input energy;
- grid import energy;
- auxiliary energy;
- storage throughput;
- generator starts.

Do not sum these into a single energy cost without an explicit boundary and double-counting analysis. In particular, storage throughput is not additional primary energy, and auxiliary energy may already be part of the POC boundary.

### Controller and recovery observations
- decision count, missed decision deadlines and p50/p95/p99 decision latency;
- time to safe state;
- recovery duration;
- time to terminal reserve restoration.

Latency quantiles retain their sample count. Their statistical adequacy and cluster/dependence treatment are outside this module and belong to [issue #434](https://github.com/Luminous-Dynamics/luminous-platform/issues/434).

## 5. Paired campaign rules

A comparison requires:
- distinct reference and candidate policy/campaign identities;
- same scenario-corpus digest;
- exactly the same scenario ID set;
- matching scenario digest for each paired case;
- identical per-case matched-input digest;
- exact policy and run identity retained for both sides.

The report preserves complete campaign and episode snapshots. It reports per-gate counts and episode disposition counts only. It does not compute a weighted score, superiority, significance, confidence interval or ranking. Failed and inconclusive episodes remain in the denominator.

## 6. Synthetic fixtures

The schema and fixture live under `schemas/dispatch-outcome-*`. The fixture intentionally includes:
- a normal service episode;
- an outage episode where a policy uses less fuel but fails critical service and reserve;
- a telemetry-unknown episode that must remain inconclusive.

These fixtures demonstrate disposition semantics only. They are not a controller execution, plant simulation or physical comparison.

## 7. Implementation boundary

The dependency-free typed module `tools/measurement-audit/src/dispatch_outcomes.rs` validates episode arithmetic and gate consistency, validates campaigns, and composes matched campaign reports. It does not parse JSON, simulate plant physics, execute policies, authenticate signatures, calculate statistical significance, or determine performance direction.

Claim ceiling: **DISPATCH_EPISODE_OUTCOME_AND_PAIRED_STRUCTURE_ONLY_NOT_SUPERIORITY_OR_CONFORMITY**.
