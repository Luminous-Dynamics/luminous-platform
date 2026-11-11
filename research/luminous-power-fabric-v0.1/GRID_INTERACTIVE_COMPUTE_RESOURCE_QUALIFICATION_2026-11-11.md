# Grid-Interactive Compute Resource Qualification v0.1

Research snapshot: 11 November 2026.

Status: **offline event-level evaluator and nineteen synthetic Rust unit tests authored; tests not executed**. This is a proposed measurement and qualification protocol, not a grid-market product, accredited capacity result, utility approval or field demonstration.

## 1. The paradigm shift

The existing Power Fabric increments compare matched operating points, preserve workload/QoS contracts and distinguish per-GPU power from fleet capacity. A new question remains:

**How much compute-derived demand reduction can a specific facility credibly commit to deliver, at a specific point of interconnection, for a specified response and sustain period, across repeated dispatch events, without violating compute service or recovery obligations?**

A maximum observed watt reduction is not a firm resource. Neither is an average of successful events if unsuccessful events disappear from the denominator. A valid commitment needs a frozen counterfactual, attributable load reduction, measurement uncertainty, response/sustain/recovery evidence and repeated-event reliability.

The report keeps three demand-response purposes separate:

- **Market-serving:** shifting or reducing demand against a market/tariff signal.
- **Grid-serving:** relieving a named network bottleneck at a declared location.
- **System-serving:** contributing demand-side flexibility to a system-level operating need.

These categories follow the framing in IEA 4E's *Data Centres and Flexibility* (1 July 2026). The evaluator does not collapse them into one scalar score; location and service purpose are part of the contract.

This is a compute-derived demand-response evaluator. It does **not** qualify inverter-based frequency response, voltage support, grid-forming behaviour, export, operating reserves or protection performance. Those require separate electrical-resource models, test procedures and authorisations.

## 2. External state of the art and the opportunity

### Workload-level flexibility

Caprara et al., *Data center workload flexibility for power system demand response: Evidence from Alibaba traces* (May 2026, DOI [10.1016/j.ijepes.2026.111940](https://doi.org/10.1016/j.ijepes.2026.111940)), presents Latency-Aware Deferral for Flexibility (LAD-Flex). Its one-week trace study uses more than one million tasks and reports up to 22% load shedding during flexibility windows, with more than 20% of estimated GPU-side power attributable to tasks with deferrable latency. The article describes its power reduction as modelled using task-trace and power-model assumptions; its data are available from the authors on request.

That is a useful reproduction comparator, not a universal flexibility percentage and not evidence of a contracted facility delivering 22% in field dispatch. The first Luminous comparison should reproduce the published scenario under the same trace, latency constraints, time window, baseline and power model before claiming an advance.

### Fleet energy and operating points

The ELF / Joule Point line establishes why a per-GPU energy minimum is not the same as a fleet or site commitment: reducing per-device goodput can require more installed GPUs and supporting capacity. The earlier Luminous fleet evaluator (#394) addresses fixed-fleet feasibility and a clearly labelled linear capacity projection. This increment adds repeated event delivery and site attribution; it does not remove the need for physical multi-GPU and site measurements.

### Grid planning is moving toward flexible-load evidence

IEA 4E's 2026 assessment explicitly separates market-, grid- and system-serving flexibility and notes operational/economic barriers to deployment. The UK Energy Networks Association portal lists the live NESO NIA3_NESO007 feasibility study (September–December 2026), which is collecting evidence on workload shifting, smart scheduling and behind-the-meter strategies. The portal describes an active study and its intended benefits; these are not completed study findings.

Lawrence Berkeley National Laboratory's May 2026 article *Integrating AI Data Centers with the Power Grid* groups the principal levers as computational flexibility, flexible facility infrastructure, storage and onsite generation. The present evaluator intentionally isolates the compute-derived component rather than crediting storage or non-compute load actions to workload scheduling.

### Measurement and verification

Demand-response measurement is fundamentally a counterfactual problem: the operator needs to estimate the energy or power that would have been used in the absence of the dispatch action, then compare it with what was actually delivered. NIST's foundational measurement-and-verification review identifies accurate determination of delivered response under imposed conditions as a core requirement.

Google's deployed carbon-aware computing system is a relevant scheduling comparator: its Virtual Capacity Curves limit hourly capacity available to temporally flexible workloads while preserving daily completion capacity. This is strong evidence that temporal workload orchestration is operationally useful, but carbon-aware scheduling is not itself proof of grid-dispatch response, local network relief or event reliability.

## 3. Counterfactual and attribution contract

Before dispatch, freeze:

- Resource, grid zone and point-of-interconnection identifiers.
- Site operating envelope and electrical measurement boundary.
- Workload, request mix, accepted-output quality and control-action identifiers.
- Counterfactual baseline identity, evidence reference, method, parameters and training data cutoff.
- Event sampling and stratification design.
- Required response, sustained delivery and recovery windows.
- Service, useful-work, uncertainty and rebound limits.

Allowed baseline methods are declared separately:

1. **Frozen workload replay:** the same trace, request mix and quality contract are replayed under a no-dispatch policy.
2. **Concurrent matched control:** an independently metered matched facility or partition estimates the no-dispatch counterfactual.
3. **Independent validated model:** a preregistered model with separate validation evidence estimates the counterfactual.

A missing, post-hoc or unqualified baseline is rejected. A baseline identifier or evidence ID is metadata, not proof that the source trace, model or validation is truthful. The underlying evidence still needs independent verification.

### Compute-only power attribution

At the site/PCC boundary:

\[
\Delta P_{\mathrm{site}} =
P_{\mathrm{site,baseline}} - P_{\mathrm{site,event}}
\]

The observed change in non-compute loads is independently measured. Positive values represent non-compute load reduction:

\[
\Delta P_{\mathrm{compute}} =
\Delta P_{\mathrm{site}} - \Delta P_{\mathrm{noncompute}}
\]

The conservative creditable reduction subtracts declared absolute uncertainty bounds:

\[
P_{\mathrm{compute,LB}} =
\Delta P_{\mathrm{compute}} -
U_{\mathrm{site}} - U_{\mathrm{noncompute}}
\]

This prevents HVAC, UPS, lighting or other non-compute reductions from being attributed to the workload action. It also prevents measurement uncertainty from being counted as dependable capacity. The API accepts conservative absolute bounds, not standard deviations; the method used to derive those bounds belongs in the upstream measurement evidence.

If non-compute channels or their uncertainty are missing, a compute-attributed capacity claim is not supported. This v0.1 evaluator requires both channel values and conservative uncertainty bounds for every event.

## 4. Event eligibility and recovery

Every dispatch event is bound to the same resource, location, point of interconnection, site operating envelope, workload/request/quality contract, measurement boundary and control action.

An event is service-feasible only when all of the following hold:

- The upstream measurement contract is marked valid and linked to an evidence ID.
- Conservative compute-attributed reduction is positive.
- Combined uncertainty is within the declared fraction of baseline site power.
- Response latency meets the service contract.
- Sustained delivery lasts at least the declared minimum.
- Useful-work retention remains above its declared floor.
- P99 latency, P99 TTFT and SLO violation remain inside their limits.
- The recovery observation is bound to explicit start/end timestamps, begins after the dispatch window, does not overlap another event or recovery interval, and is long enough to cover the declared horizon.
- Rebound energy is actually measured and bounded.

Energy shifted into the recovery window is not free flexibility:

\[
E_{\mathrm{shed,LB}} =
P_{\mathrm{compute,LB}} \cdot T_{\mathrm{sustain}}
\]

\[
r_{\mathrm{rebound}} =
\frac{E_{\mathrm{rebound}}}{E_{\mathrm{shed,LB}}}
\]

If the rebound observation is missing, or the ratio exceeds the contract maximum, that event receives zero credited capacity. A lower power trace with reduced useful work or an unobserved recovery period cannot become a successful event.

The evaluator rejects overlapping dispatch/recovery intervals for one resource, recovery windows that begin before dispatch ends, duplicate event IDs and events that alter the frozen location, boundary, workload or control contract.

## 5. Repeated-event capacity and reliability

The evaluator reports three distinct quantities:

1. **Empirical event success fraction:** commitment-success events divided by all submitted events. Failed events remain in the denominator.
2. **P10 credited compute capacity:** nearest-rank 10th percentile of event-level credited capacity. A service-failed event contributes zero. A service-feasible event that delivers less than the commitment retains its smaller observed capacity and still counts as a failed commitment event.
3. **One-sided Wilson lower confidence bound:** a conservative interval estimate for the event-success probability using the declared z-score. For a one-sided 90% lower bound, use z = 1.2815515655; z = 1.644853627 is the one-sided 95% value, not the 90% value.

Qualification requires all of the following:

- Event count meets the contract's minimum.
- P10 credited capacity is at least the declared commitment.
- The Wilson lower bound meets the required success probability.

With 30 independent events and all 30 successful, the one-sided 90% Wilson lower bound is approximately 0.9481. With 29 successes from 30 events it is approximately 0.8949, so one failed event misses the initial 0.90 reliability gate. A one-sided 95% Wilson lower bound of at least 0.95 requires at least 52 independent all-success events; any failures require a larger sample, and representative event design remains essential.
- The event design declares the independent-trial assumption.

**Statistical limitation:** the Wilson bound has its stated binomial interpretation only when the event trials are representative and sufficiently independent. Repeated events on the same day, under the same weather or thermal state, or under correlated workload conditions may not be independent. The code withholds the confidence bound when the independent-event assumption is not declared, but a boolean declaration and a design-reference string do not prove independence. A field programme should use preregistered randomization/stratification, cluster-aware analysis or a hierarchical model where appropriate, and independent statistical review.

P10 is an empirical sample statistic, not a certified dependable capacity, a resource-adequacy accreditation or a market capacity product. Results are conditional on the tested site, point of interconnection, grid zone, workload, season/thermal envelope, hardware, control implementation and event sampling design. The v0.1 contract intentionally does not extrapolate from one site to a fleet or another grid node.

## 6. v0.1 implementation

The Rust module at tools/measurement-audit/src/grid_service.rs exposes:

- GridServiceContract — location, operating envelope, service class, frozen compute contract, baseline method, event-sampling assumptions and explicit qualification gates.
- GridDispatchEvent — site and non-compute power deltas, conservative uncertainty bounds, compute service metrics, response/sustain and recovery observations.
- evaluate_grid_service_sweep — per-event attribution, failed-event zero credit, nearest-rank P10 capacity, event success fraction and conditional Wilson lower bound.
- GridServiceQualificationReport — the exact frozen contract and thresholds, baseline IDs/evidence, sampling design, P10 capacity, reliability, event counts, and each qualification gate.

The evaluator rejects finite input combinations that overflow when deriving site reduction, compute attribution, uncertainty, conservative shed energy or rebound ratio. The standalone Wilson helper also withholds a result for an overflowed z-score.
- GridDispatchEventResult — event evidence ID and exact dispatch/recovery timestamp intervals alongside attribution and pass/fail metrics, so credited capacity remains traceable.

Sixteen synthetic Rust unit tests are authored. They exercise the all-pass repeated-event case and negative controls for single-event overclaim, one failed event, non-compute subtraction, uncertainty, missing/excessive rebound, missing recovery, useful-work loss, P99/TTFT/SLO regression, late response, short sustain, post-hoc baselines, unvalidated baseline models, unproven event independence, changed location/boundary/workload/control, overlapping windows, duplicate IDs and invalid measurement contracts.

**Tests are authored but not executed.** Compilation, cargo test, cargo fmt, cargo clippy, live dispatch, scheduler/GPU control, site metering, independent baseline validation, field trial, grid participation, capacity accreditation and certification are not claimed.

## 7. Proposed benchmark targets

These are initial research gates, not grid-code requirements, standards, market rules or achieved Luminous performance.

| Axis | Initial research gate | Why |
|---|---:|---|
| Repeated trials | At least 30 non-overlapping dispatch events | Prevents a best-event demonstration from becoming a commitment |
| Conservative capacity | P10 credited compute shed >= declared commitment | Avoids sizing a commitment on the mean or maximum |
| Reliability | One-sided 90% Wilson lower bound >= 0.90 | Tests event-to-event dependability under the stated binomial assumption |
| Useful work | >=99% retention | Prevents load shedding by simply failing to do the work |
| Service | P99 latency and TTFT increase <=5%; SLO violation <=1% | Keeps flexibility inside the frozen service contract |
| Response and sustain | <=1 s response; >=60 s sustain in the synthetic profile | Makes the test executable; replace with the actual grid product's requirements |
| Recovery | >=300 s observed; rebound <=10% of conservative shed energy | Exposes post-event payback |
| Measurement | Combined uncertainty <=5% of baseline site power | Prevents uncertainty dominating credited watts |

A future production claim should specify the actual market/grid service product, jurisdiction, response and sustain durations, metering and telemetry rules, counterfactual method, statistical confidence, test population, seasonal coverage, failure handling and independent assurance. A 95% one-sided confidence claim should use a preregistered sample-size calculation and analysis method; it must not be inferred from the 30-event research fixture.

## 8. How to compare and beat current practice

| Comparator | What it establishes | Luminous comparison / advance target |
|---|---|---|
| Caprara et al. LAD-Flex (2026) | Up to 22% load deferral in a one-week Alibaba-trace case study | Reproduce on the same permitted traces and assumptions; report service-preserving P10 repeated-event capacity, not only maximum modeled shed |
| ELF / Joule Point | Measured GPU power-cap/DVFS operating points and fleet energy/throughput trade-offs | Retain matched per-GPU and fixed-fleet results, then demonstrate event-level site delivery, uncertainty, recovery and counterfactual attribution |
| Google Carbon-Intelligent Compute Management | In-production temporal capacity curves that preserve daily work completion | Compare forecasted versus actually delivered shiftable capacity, then add location-specific dispatch compliance and post-event backlog/rebound |
| IEA 4E and NESO direction | Market-, grid- and system-serving flexibility are distinct and deployment needs operational/economic evidence | Publish separate service envelopes, point-of-interconnection evidence and cost/availability limits |
| NIST demand-response M&V | Delivered response depends on fair, accurate counterfactual measurement | Make baseline freeze, non-compute subtraction, uncertainty and independent validation machine-checkable |

The research opportunity is not to claim a higher percentage than every paper. It is to combine the best pieces into a testable resource contract: **compute service + attributable site watts + location + uncertainty + response + sustain + recovery + repeated-event reliability**.

The strongest competitive result would be an independently reproduced site pilot showing that the P10 conservative capacity meets its declared commitment over a preregistered event population, while preserving accepted useful work and customer QoS. Publish the trace, control, baseline method, raw measurement boundary, uncertainty budget, failed events and recovery window to the extent rights allow. A result that cannot be independently replayed should not be presented as better than SOTA.

## 9. Scope and safety

This module is read-only. It does not send commands to workload schedulers, GPUs, UPSs, batteries, converters, generators, markets, aggregators or grid operators. It does not calculate inverter response, protection behaviour or interconnection compliance.

No grid-connected test, 800 V energization, storage fault injection, market bidding, resource accreditation, product safety approval or certification is authorized or claimed. Local operators, grid codes, interconnection agreements, protection systems and accredited test processes remain authoritative.

## Sources

- IEA 4E, [Data Centres and Flexibility](https://www.iea-4e.org/edna/publications/data-centres-and-flexibility/), 1 July 2026.
- A. Caprara et al., [Data center workload flexibility for power system demand response: Evidence from Alibaba traces](https://doi.org/10.1016/j.ijepes.2026.111940), *International Journal of Electrical Power & Energy Systems*, May 2026.
- Energy Networks Association / NESO, [Flexible Operation of Data Centres Feasibility Study, NIA3_NESO007](https://smarter.energynetworks.org/projects/nia3_neso007/), September–December 2026 project period.
- J. Granderson et al., [Integrating AI Data Centers with the Power Grid](https://doi.org/10.20357/B7X61S), Lawrence Berkeley National Laboratory, May 2026.
- D. G. Holmberg, D. Hardin and E. Koch, [Towards Demand Response Measurement and Verification Standards](https://www.nist.gov/publications/towards-demand-response-measurement-and-verification-standards), NIST, 2013.
- [Carbon-Aware Computing for Datacenters](https://doi.org/10.1109/TPWRS.2022.3173250), *IEEE Transactions on Power Systems*, 2023.
