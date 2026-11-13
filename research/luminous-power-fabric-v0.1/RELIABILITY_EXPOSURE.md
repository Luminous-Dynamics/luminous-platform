# Exposure-normalized reliability events and service impact — v0.1

**Tracking:** [Issue #471](https://github.com/Luminous-Dynamics/luminous-platform/issues/471)  
**Temporal evidence:** [draft PR #469](https://github.com/Luminous-Dynamics/luminous-platform/pull/469)  
**Paired lifecycle inference:** [draft PR #463](https://github.com/Luminous-Dynamics/luminous-platform/pull/463)

**Status:** read-only exposure and event accounting with synthetic fixtures. No safety case, physical reliability or failure-probability claim is made.

## Why a better denominator matters

EPRI's October 2026 report *Risk Factors and Failure Statistics of Lithium Ion BESS* shows seven US grid-scale failure incidents in 2025 and approximately 0.04 incidents per deployed GWh, with cumulative deployment near 148 GWh. This is important fleet context, but cumulative deployed GWh is not the same as energized asset-hours, installed MW-years, dispatch exposure or cycle throughput. Do not compare Power Fabric directly against that figure unless populations and denominator definitions match. See the [EPRI report](https://restservice.epri.com/publicdownload/000000003002034920/0/Product).

EPRI's RELIABLE programme identifies limited installed performance/non-performance data and inconsistent field collection as barriers to comparable fleet reliability benchmarking: https://storagewiki.epri.com/index.php/Battery_Energy_Storage_Roadmap/RELIABLE

The design complements, rather than replaces, post-commissioning measurements in [IEC TS 62933-2-3:2025](https://webstore.iec.ch/en/publication/77334) and field-health indicators in [IEA PVPS Task 13's July 2026 report](https://iea-pvps.org/key-topics/assessing-reliability-battery-systems-plants-2026/).

## Paradigm: every event is conditioned on exposure

Confirmed-failure incident counts are reported separately per energized asset-hour, installed asset MW-year, charged MWh, discharged MWh, equivalent full cycle, dispatch attempt and source-transfer attempt. The denominators are not interchangeable. Every rate retains its numerator, denominator value/unit, normalization factor and null status. A zero denominator never becomes zero risk or an infinite rate.

One input is one matched exposure cohort: population, site profile, topology, configuration and system/component asset scope must be consistent. Overlapping exposure windows for the same asset are rejected to prevent denominator duplication. A component-hour rate must not be relabelled as a system-hour rate.

## Event semantics and severity

Each event binds stable event/incident IDs; subsystem, failure mode, detection source, affected component and configuration; onset/detection/containment/restoration TAI intervals from the temporal-evidence owner; electrical/operating context; unserved critical Wh, interruption seconds, reserve shortfall and repair/requalification effort; independent severity dimensions; root-cause confidence, common-cause group and evidence references.

Multiple event records in one incident group remain visible but count once in incident rates. A single incident group cannot contain contradictory classes. Severity is reported separately for personnel safety, fire/thermal, electrical, environmental, service interruption, equipment damage and regulatory reportability. There is no scalar severity score. Unknown severity remains visible and blocks full physical claim eligibility.

Near misses and precursors are not counted as confirmed failures. Planned and unplanned outage counts are separate. Overlapping outage intervals are unioned per asset; restored timestamps produce lower/upper downtime bounds. Open outages are counted through evaluation time rather than dropped.

## Reliability metrics

The evaluator reports rates per 1,000 energized asset-hours, installed asset MW-year, 1,000 charged MWh, 1,000 discharged MWh, 1,000 equivalent full cycles, 10,000 dispatch attempts and 1,000 source-transfer attempts. It also reports near-miss/precursor counts, planned/unplanned outage counts, common-cause groups, root-cause confidence counts, severity counts, unserved critical Wh, reserve shortfall, repair/requalification labour, unplanned repair-duration p50/p90/p95 bounds and planned/unplanned/total asset-unavailable hour bounds.

An incomplete event-history declaration or missing/assumed/non-physical event, timebase or exposure evidence prevents physical reliability claim eligibility. Evidence fields are checked for declared status/class and references only; this module does not verify signatures or authenticate raw data. Mycelix owns that trust boundary.

## Research benchmark targets

- 100% incident counts tied to stable incident-group and immutable event records.
- 100% denominator windows tied to asset/population, system/component scope, state/mode, site, topology/configuration, timebase and evidence.
- Zero overlapping exposure-window double counting.
- Zero omission of near misses, precursors, open outages or unserved critical service.
- 100% rate output includes numerator, denominator, unit, normalization factor and rate/null.
- Zero pooling of different site, topology, configuration, population or asset scope in one cohort.
- Severity remains multidimensional; no financial or availability offset may compensate for a serious safety event.
- Common-cause and root-cause uncertainty remain visible.
- TAI intervals are consumed from the temporal-evidence contract; point timestamps cannot manufacture causality from overlapping bounds.

## Limits and next steps

This version provides accounting only. It does not estimate Poisson/Bayesian hazard rates, repairable-system reliability, confidence intervals, causal fault probabilities, fire risk, safety certification or future RUL. Common-cause dependencies and cluster structure mean a naive independent-event Poisson model could be misleading.

Next: stratified cohort outputs by configuration/site, then link paired cluster inference only after the reliability/statistics owner validates the sampling unit and event-history completeness.