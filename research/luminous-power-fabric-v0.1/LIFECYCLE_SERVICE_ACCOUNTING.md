# Lifecycle service accounting: cost, downtime and observed storage health

**Snapshot:** 12 November 2026  
**Tracking:** [#427 — lifecycle service ledger](https://github.com/Luminous-Dynamics/luminous-platform/issues/427)  
**Parent research:** [#424 — lifecycle-adjusted useful-service benchmark](https://github.com/Luminous-Dynamics/luminous-platform/issues/424)  
**Implementation:** research/lifecycle-service-ledger-v0.1  
**Evidence status:** deterministic nominal accounting and synthetic fixtures only. No lifetime, remaining-useful-life, degradation forecast or product advantage is claimed.

## 1. Paradigm shift

The unit of competition should not be purchase price, nominal battery kWh or one-cycle efficiency. It should be **cost per delivered critical-service kWh and cost per completed, matched mission**, with repair downtime, replacement, requalification and measured end-of-period capability visible.

This does not require the platform to create a new electrochemical ageing model. It requires the platform to reconcile:
- the same scheduled critical service and terminal-state obligation;
- actual delivered and unserved critical service over repeated missions;
- itemized cost transactions in a declared currency;
- repair and requalification time, replacement identity and unavailable-service intervals;
- repeated, evidence-qualified storage capacity and discharge-power measurements.

A low-cost candidate that misses critical service or cannot prove its end reserve does not win by dividing its cost by a nominal kWh figure. Failed missions remain in the record.

## 2. Owner boundaries

| System | Owns | Platform consumes |
|---|---|---|
| Symtropy | Generator/fuel histories, source dynamics, duty history and owner-validated physical models | Actual source-energy/fuel and operating-history records with model/test revision |
| Mycelix | Instrument/channel identity, calibration, timebase, uncertainty, SOC/SOH/capacity/power observations, invalidation | Validity and independent measurement status; no duplicate metrology registry |
| Sol-Atlas | Site/environment exposure, repair access, spare logistics, deployment geometry and local service context | Site-profile revision and location-bound logistics evidence |
| Symthaea | Information-gain selection of accelerated-life, repair and field experiments | Ranked test proposals only; no evidence or safety authority |
| Platform | Matched mission contract, cost/service aggregation, downtime union, component identity/replacement and comparison report | Typed owner-native outcomes and evidence references |

The platform does not infer battery chemistry, fit an ageing law, forecast remaining useful life, generate a fuel map or calculate service-logistics travel time from coarse location labels.

## 3. Cost ledger and denominators

Each cost record carries:
- unique ledger entry ID and a separate **economic-event ID** to prevent the same invoice/transaction being entered under multiple line items;
- one category: initial capital, fuel, purchased energy, cooling/auxiliaries, maintenance parts, maintenance labour, replacements, service logistics, disposal or other;
- non-negative integer minor currency units, currency code and declared minor-unit scale;
- incurred timestamp, evidence status/class and evidence reference.

Money stays in integer minor units when totals are calculated. v0.1 does not perform FX conversion, inflation adjustment, tax normalization, NPV/discounting or future price forecasts. Comparators must use the same currency and cost-allocation boundary before comparing ratios.

For delivered critical energy E_delivered (Wh) and total included cost C_minor:

C_minor_per_critical_kWh = C_minor × 1000 / E_delivered

For N_completed successful missions:

C_minor_per_completed_mission = C_minor / N_completed

A failed mission is not removed from total cost or from the mission count. Cost per completed mission is undefined if none completed. Cost per delivered kWh is undefined if no critical energy was delivered. The report retains total required, delivered and unserved critical energy, mission count and success rate beside the ratios; the ratio alone is never a pass metric.

Actual cost evidence is distinguished from estimates. Invoice/receipt, utility bill and payroll/work-order classes can support an actual-cost claim when current. Quotes, estimates, vendor declarations and synthetic fixture values can still be summed for planning, but cannot become actual cost evidence merely because they have a number.

## 4. Repair and availability accounting

A repair event identifies the component, replacement component (if any), pre/post repair configuration digests, system-unavailable interval, repair labour, requalification time and evidence.

The study carries an explicit scheduled **operating window** separately from the calendar study timestamps. Mission and repair intervals use the operating-time axis. Availability is:

Availability = (scheduled_operating_time - union(system_unavailable_intervals)) / scheduled_operating_time

Overlapping repairs are merged before downtime is summed. Two concurrent module repairs that disable the same system for 30 minutes count as 30 minutes of system downtime, not 60 minutes. Repair labour and requalification labour are separately summed, since two technicians can work simultaneously while system downtime is counted once.

The study must also carry an explicit repair-history-complete flag, evidence status/class and reference. An empty repair-event list without a complete maintenance/service record is not evidence that there were no repairs. Replacement requires a distinct incoming component identity, unique across the study, and explicit pre/post configuration digests. A replacement module starts a new physical health history; it is never spliced into the outgoing module's trajectory.

## 5. Storage health is observed, not forecast

Each event-time storage observation binds:
- exact physical component identity and qualified/observed configuration digest;
- absolute observation timestamp and timebase reference;
- usable capacity and its absolute uncertainty bound;
- maximum discharge power and its uncertainty bound;
- cumulative energy throughput, calendar age and equivalent full cycles;
- SOC, SOH and temperature;
- evidence, calibration and sensor states;
- operating-condition and measurement evidence references.

For the same component and unchanged configuration, with at least two chronologically ordered, current, calibrated, sensor-valid, independently measured observations, the accountant computes conservative lower-bound retention:

Capacity retention = last(usable capacity - capacity uncertainty) / first(usable capacity - capacity uncertainty)

Power retention = last(max discharge power - power uncertainty) / first(max discharge power - power uncertainty)

Throughput, calendar-age and equivalent-full-cycle deltas are reported with first/last SOC/SOH/temperature, timebase and evidence references. Throughput, calendar age and cycle counts must be non-decreasing. A configuration change, expired/invalid calibration, vendor-only report, missing point or replaced component prevents an independent retention trajectory from being claimed.

This is a first-to-last observed ratio, **not a forecast**, degradation curve, chemistry model, remaining useful life or warranty estimate. Different temperature/SOC/C-rate test conditions must not be treated as a controlled degradation comparison unless the measurement programme declares and controls them.

## 6. Strict matched-comparison contract

Reference and candidate must match:
- comparator-set ID, currency/minor-unit scale and cost-allocation boundary;
- site-profile digest and useful-service boundary;
- calendar study horizon and scheduled operating window;
- mission IDs and mission-profile digests;
- each mission's start/duration, critical-service demand, permitted local unserved-energy allowance, initial-state-obligation digest and terminal-state-obligation digest, plus explicit references to the actual initial and final state observations.

Their product configurations, costs, fuel, repairs, replacements and measured health trajectories are allowed to differ; those are the variables being compared. Initial state and terminal reserve obligations are separately bound to different digests and both must match. Each mission also retains distinct references to the observed initial state and observed final state; an obligation hash alone is not a measurement. A mismatch in the requested service or final reserve contract rejects the comparison before calculating a candidate delta.

The output's claim eligibility is intentionally stricter than its ability to calculate arithmetic. A physical lifecycle comparison requires:
- eligible exact system configuration and current qualification;
- current cost evidence and actual transaction evidence classes;
- current mission evidence with independent physical measurements;
- complete, current repair history with physical work-order/service evidence;
- every mission to pass its energy, event and terminal-state screens;
- where storage is in scope, complete repeated independent health observations for every physical storage component in scope.

Synthetic fixture records are never eligible for a physical lifecycle claim.

## 7. Synthetic comparator illustration

The fixture file `schemas/lifecycle-service-accounting.synthetic.json` contains two deliberately synthetic, matched comparators:
- generator-only reference total cost: 1,050,000 USD minor units;
- modular hybrid candidate total cost: 972,000 USD minor units;
- each delivers 1,000 Wh critical service in one synthetic completed mission;
- candidate repair intervals overlap, with one synthetic component replacement;
- both have two synthetic storage-health observations for one identified component.

Arithmetic-only candidate cost change is approximately -7.43% per delivered critical kWh. The candidate also has 450 seconds of unioned system downtime in the 3,600-second scheduled operating window (87.5% availability), while the reference has none. The fixture's health evidence class is synthetic, so neither scenario can claim physically measured retention or lifecycle superiority. These numbers are not vendor quotes, invoices, field observations or a Power Fabric performance result.

## 8. Benchmark programme

1. **Accounting oracle:** duplicate economic-event rejection, integer minor-unit sums, category order invariance, cost/service denominators, zero-completed mission, service failures and reserve/event failure.
2. **Repair oracle:** overlapping and adjacent outage intervals, separate labour/requalification, complete-history evidence, component replacement identity and pre/post digests.
3. **Health observation oracle:** lower-bound capacity/power retention; repeated independent measurement; monotonic throughput/calendar/EFC; configuration change, module replacement, non-current calibration, BMS-only report and missing observation.
4. **Matched comparison oracle:** identical mission/profile/terminal obligations and boundary; reject mismatched currency, site, calendar horizon, operating window and mission set.
5. **Physical programme:** repeated matched service missions on an incumbent and candidate; calibrated energy/fuel/cost/health evidence, controlled SOC/SOH/temperature/C-rate conditions and independent review.
6. **Lifecycle result:** report cost per critical kWh, cost per completed mission, service shortfall, reserve failures, uptime, repair/requalification, replacements, material/spares, measured retention and uncertainty together.

No arbitrary superiority percentage is frozen before a matched repeated-mission dataset exists. Any margin must be preregistered and greater than the combined uncertainty and run-to-run variation. Efficiency or cost cannot compensate for a critical-service, reserve, protection or safety failure.

## 9. Explicit claim ceiling

The Rust implementation at `tools/measurement-audit/src/lifecycle_service_cost.rs` computes nominal cost/service ratios, outage interval unions and conservative first-to-last observed storage retention. It does not perform:
- NPV/discounting, inflation/FX, tax or future-price forecasting;
- capacity-fade interpolation, degradation-law fitting, RUL or lifetime prediction;
- fuel/engine or battery electrochemistry modelling;
- safety, battery transport, product conformity, protection or field qualification.

The current Rust tests are authored controls. They are not claimed as executed until a Rust toolchain and exact-head CI results are available.
