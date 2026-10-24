# Field Reliability and Recovery Benchmark — DSE-002

**Version:** 0.1  
**Status:** Protocol proposal; no prototypes or laboratory results reported  
**Parent study:** FIELD_COMPUTING_DSE_001.md  
**Tracking:** Luminous-Dynamics/luminous-platform#128 and draft PR #129

## 1. Research thesis

Conventional ruggedness and repairability evaluations often optimize different things. The proposed benchmark evaluates the combined system: damage prevention, inexpensive recovery, post-repair environmental integrity, software recovery and lifecycle cost.

The primary outcome is not a one-time drop height. It is **successful service delivered per lifecycle cost**, including repair delay.

`availability = operating_time / (operating_time + unplanned_downtime)`

`cost_per_successful_hour = total_lifecycle_cost / verified_operating_hours`

Report both values and their underlying components. A single weighted score may hide important trade-offs, so publish raw results and Pareto frontiers.

## 2. Product classes and matched baselines

Use two explicit comparison classes; do not compare a thick, expensive industrial unit to a slim consumer device without stating the difference.

### Smartphone class

- Ruggedness baseline: a current Panasonic TOUGHBOOK/Getac handheld or a currently available rugged Android device whose exact model and claimed test profile are recorded.
- Repairability baseline: Fairphone Gen 6, which currently advertises 12 user-replaceable parts, display/battery/USB-C/back-cover swaps in under five minutes, and parts availability until 2033.
- Proposed Luminous prototype: matched at a declared target for weight, screen size, battery capacity and price class, with raised bezel, replaceable sacrificial front component and serviceable display cassette.

### Laptop class

- Ruggedness baseline: an exact Panasonic TOUGHBOOK/Getac model with the manufacturer's environmental test documentation and optional battery/configuration recorded.
- Repairability baseline: Framework Laptop 13 or another exact repairable Linux-capable platform. Framework's published guides include display, bezel, hinge, keyboard, memory, storage, battery, Wi-Fi, mainboard and port-related service operations.
- Proposed Luminous prototype: a repairable Linux/NixOS compute platform with modular enclosure and ports, tested in the same mass/configuration class wherever possible.

Baseline reference pages (re-check the exact model, revision, purchase date and claims when the test is commissioned):
- Fairphone Gen 6: https://www.fairphone.com/the-fairphone-gen-6
- Fairphone Gen 6 support and repair: https://support.fairphone.com/hc/en-us/articles/24590029621138-The-Fairphone-Gen-6-Replace-a-Spare-Part
- Framework Laptop 13 repair guides: https://guides.frame.work/c/Framework_Laptop
- Panasonic TOUGHBOOK selection guide: https://connect.na.panasonic.com/toughbook/how-to-choose-rugged-laptops-tablets
- FAISS binary index guide is unrelated to device tests; do not reuse software performance evidence as hardware reliability evidence.

These manufacturer claims are starting points, not independent matched-test results. The benchmark must distinguish supplier specifications, lab-observed results, field observations and modelled costs.

## 3. Define failure before testing

### Smartphone failure criteria

Count as a display/system failure if any of the following occur:
- display glass or inner panel cracks, leaks, develops lines or becomes unsafe to touch;
- touch coverage or touch accuracy falls outside the published acceptance test;
- a functional area fails beyond the pre-registered dead-pixel/brightness criterion;
- device will not boot, core communications fail, battery is damaged or the display assembly disconnects;
- water/dust exposure after the test produces a safety fault, corrosion or a failure in the core-function suite.

A cracked, replaceable sacrificial protector with an intact functional display is not a device failure. Record it as a protection-consumable event with part cost and replacement time. Do not hide it as a success without reporting the damage.

### Laptop failure criteria

Count a system failure if:
- display fracture or loss of required image/touch function;
- hinge, chassis, keyboard, port or battery damage prevents the defined core workflow;
- SSD/storage loses data or cannot be recovered using the published recovery procedure;
- the system cannot reach a known-good NixOS environment using the documented offline recovery path;
- ingress/thermal stress produces a safety issue or persistent core-function failure.

Record degraded performance, stuck keys, port failures, enclosure damage and recoverable OS faults separately; they may not all have the same severity but should not disappear from the report.

## 4. Test matrix

Freeze the protocol before the qualification lot. The selected methods and severities must reflect a declared user life-cycle profile; there is no single universal rugged-device drop test.

| Sequence | Purpose | Records required |
|---|---|---|
| Baseline function and calibration | Establish known-good state | Serial/build ID, mass, software digest, display/touch tests, port/battery health |
| Multi-orientation drop | Expose face, edge, corner, rear and hinge/display failure modes | Surface, height, orientation, impact count, high-speed video or accelerometer if available |
| Repeated impact | Detect damage accumulation and loose connectors | Functional suite after every event, not merely at the end |
| Torsion/bending/point load | Find glass and chassis failure outside ideal face drops | Applied fixture force/displacement and crack/flex observations |
| Dust and water ingress | Check seals and port covers | Exact IEC 60529 test profile when claiming an IP rating; post-test internal inspection |
| Thermal cycling and operation | Detect seal, adhesive, battery and connector problems | Temperature profile, dwell, condensation conditions, boot/function results |
| Transport vibration and shock | Model backpack, vehicle and workshop transport | Axis, spectrum/profile, duration, fixture data and post-test functional status |
| Port, hinge and key cycles | Measure wear of high-use mechanical interfaces | Cycle count, force, intermittent faults, module replacement time |
| Repair and reseal | Measure repairability under real service conditions | Technician, tools, repair time, part cost, errors, retest outcomes |
| Offline recovery | Test data and operating-system recovery | Time to known-good operation, network availability, data loss, source/config digest |
| Sustained workload | Ensure ruggedness additions did not create thermal/power regression | Named workload, input digest, sustained performance, throttling, battery/runtime and calibrated energy where available |

For ingress, use IEC 60529 methods appropriate to the actual product and intended claim. Re-test after repair cycles that disturb seals. MIL-STD-810H should be tailored to the stated operating and transport profile; a generic mention of the standard is not evidence that a device passed any method.

References:
- US DLA MIL-STD-810H details: https://quicksearch.dla.mil/qaDocDetails.aspx?ident_number=35978
- IEC 60529 publication: https://webstore.iec.ch/en/publication/2447

## 5. Screen-stack design experiment

Before designing a complete phone, compare front-stack concepts using identical off-the-shelf display modules and a controlled chassis fixture.

**Candidate variants**
- A: strengthened display glass plus replaceable external protector and raised bezel.
- B: separate replaceable sacrificial outer lens, if optical/touch/sealing stack makes this viable.
- C: serviceable display cassette with compliant perimeter mounting and replaceable corner bumpers.

Keep display panel, chassis size, battery mass, impact surface, drop orientation and test order controlled. Randomize test order and blind the person scoring display damage where practical. Include an unprotected or current-device control where safe and meaningful.

Measure:
- inner display failures per impact sequence;
- protector/outer-lens damage and replacement cost;
- peak acceleration at the display mounting points if instrumentation permits;
- repair time and whether the repair compromises sealing;
- optical clarity, touch accuracy and outdoor readability before and after damage;
- total mass, thickness, heat transfer and material cost.

Do not optimize only average drop height. A configuration that protects face impacts but fails under corner impacts is a failure of the declared profile. Publish results by orientation and failure mode.

## 6. Repair benchmark

Run service trials with technicians who did not design the product. Provide only the published repair instructions and standard tool kit. Randomize device/repair order and record time from diagnosis start to verified return-to-service, including teardown, diagnosis, replacement, reassembly and functional retest.

For every common repair, report:
- median, p90 and maximum time-to-repair;
- repair success rate without motherboard replacement;
- part price, consumables and required tools;
- number of connectors/fasteners removed;
- new fault rate after reassembly;
- ingress and function retest result;
- part availability and dispatch/arrival delay.

Target hypotheses from DSE-001—display/keyboard/battery/port replacement in ten minutes and at least eight years of parts availability—remain design goals, not achieved or guaranteed outcomes. Report actual failures and misses.

## 7. Statistical discipline: what a small pilot can and cannot prove

Separate design exploration from reliability qualification.

- **Prototype exploration:** roughly 5–10 units per variant may expose obvious failure mechanisms and inform redesign. It does not support a strong low-failure-rate claim.
- **Engineering verification:** use a preregistered set of orientations and repair cycles, plus replicates sufficient to compare configurations and observe repeatability.
- **Reliability qualification:** choose the lot size from the failure probability the product intends to bound and the confidence level. With zero observed failures in n independent representative trials, the rule-of-three approximation gives a 95% upper bound near 3/n. Thirty zero-failure trials only bound the failure probability to roughly 10%; demonstrating below 1% requires about 300 zero-failure independent trials, and below 0.1% roughly 3,000.
- Repeated drops on the same device are not independent devices. A sequence can be a valid qualification protocol but must not be counted as if every drop were an independent sample for the failure-rate estimate.
- Field events are often correlated by user, environment, manufacturing lot and usage. Report the sampling unit and stratify results rather than relying blindly on the rule of three.

This statistical limit is a central product-design constraint: use a small pilot to find and fix mechanisms, not to advertise an unrealistic reliability percentage.

## 8. Lifecycle economics

Use an explicit model whose terms can be audited:

`LCC = purchase + protection_consumables + parts + labor + logistics + energy + data_loss_cost + end_of_life - residual_value`

`service_cost_rate = LCC / verified_operating_hours`

`availability = verified_operating_hours / (verified_operating_hours + unplanned_downtime)`

Publish sensitivity ranges for repair frequency, spare-parts stockout duration, technician cost and device service life. Do not invent average break rates: measure field rates or identify the source and uncertainty of the estimate. Price comparisons must use the same region, currency/date, tax/shipping assumptions and warranty coverage.

Energy is measured at the device boundary on an identified instrument and workload. Battery size, CPU TDP and estimated workload energy are not interchangeable with measured joules.

## 9. Frontier and go/no-go

Publish separate Pareto frontiers for:
- mass vs. impact survival;
- initial cost vs. lifecycle service-cost rate;
- repair time vs. post-repair ingress success;
- battery endurance vs. sustained performance;
- failure rate vs. reliability evidence/sample size;
- repairability vs. enclosure/thermal/ingress performance.

A design advances only if:
1. it passes the declared safety and core-function criteria;
2. every claimed IP / environmental rating has suitable evidence;
3. repairs do not silently invalidate the ingress claim;
4. the measured lifecycle-cost or availability improvement is material against a matched baseline;
5. all failures and deviations are retained in the report;
6. the product remains supportable from stocked, traceable components.

Do not claim “unbreakable,” “better than all rugged laptops,” or a universal screen survival rate. State the exact conditions, sample, build and uncertainty.

## 10. First executable work sequence

1. **Select representative baseline hardware:** one Fairphone Gen 6 repairability reference and one current rugged phone for phone comparisons; one Framework Laptop 13 repairability reference and one rugged laptop for laptop comparisons. Record exact SKUs, mass, display size, battery and published test profiles.
2. **Build the screen-stack fixture:** off-the-shelf display, adjustable bezel height, swappable corner geometry and replaceable front layer; no custom radio/mainboard.
3. **Pre-register the drop/repair protocol:** orientations, surface, order, core-function suite, failure definitions, sample count and reporting format.
4. **Run an exploratory design-of-experiments pilot:** use independent units per configuration, randomize order, measure failures and consumable replacement cost, then redesign based on failure mode.
5. **Prototype the laptop enclosure around an existing repairable compute platform:** validate bumper geometry, display protection, I/O replacement, thermal paths and offline recovery.
6. **Scale to qualification only after the design stops changing.** Independently test ingress and declared environmental profiles; publish raw results and build identity.

## 11. Evidence boundary

This is a test protocol, not a test report. No prototype, drop result, reliability rate, IP rating, MIL-STD qualification, repair-time result or unit-cost estimate is claimed. Industry product pages are vendor specifications; independent comparative claims require the matched tests defined here.
