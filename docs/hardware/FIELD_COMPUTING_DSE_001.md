# Field Computing DSE-001 — Rugged Smartphone and Serviceable Field Laptop

**Status:** Design study / requirements hypothesis; no hardware prototype or qualification result exists yet  
**Tracking issue:** Luminous-Dynamics/luminous-platform#128  
**Goal:** Minimize lifetime downtime and repair cost in real field conditions, rather than maximize a single advertised drop height.

## 1. Thesis: design for survival *and* recovery

A literal guarantee that a screen can never break is not physically defensible for a portable device exposed to arbitrary impacts. A better product promise is measurable: protect the expensive display behind a sacrificial outer layer and raised frame, test the entire device against a published impact sequence, and make the damaged module inexpensive and fast to replace.

The paradigm shift is to treat durability and repairability as one reliability system. A unit that survives most drops but takes weeks and a high price to repair may be worse in the field than a unit that rarely fails and restores service in minutes.

**North-star metric:** successful operating hours per total lifecycle cost, with downtime and repair latency explicitly included.

## 2. Competitive baselines to beat

The opportunity is at the intersection of established strengths—not by pretending no incumbent exists.

| Baseline | Current documented direction | Gap our study should test |
|---|---|---|
| Panasonic TOUGHBOOK / Getac rugged computers | Vendor product families publish tailored MIL-STD-810H, ingress, drop, sunlight-readable display and hot-swap battery specifications | Do not stop at a certification label; publish our exact test profile, repair steps, part prices and failure records |
| Fairphone Gen 6 | Display and other parts are replaceable; the vendor documents screen replacement; product information advertises 12 replaceable parts and a long software/spares horizon | Pair consumer-friendly repairability with a substantially more protective mechanical design and field-qualified sealing |
| Framework Laptop family | Modularity, field-replaceable components, repair documentation and configurable expansion are core differentiators; iFixit publishes repairability assessments | Add rugged enclosure, environment tolerance and drop qualification without abandoning replaceability |

Sources:
- Panasonic rugged-device selection guide: https://connect.na.panasonic.com/toughbook/how-to-choose-rugged-laptops-tablets
- Panasonic TOUGHBOOK family: https://connect.na.panasonic.com/toughbook/rugged-laptops-tablets
- Fairphone Gen 6 repair steps: https://support.fairphone.com/hc/en-us/articles/24590029621138-The-Fairphone-Gen-6-Replace-a-Spare-Part
- Fairphone Gen 6 product/repairability details: https://www.fairphone.com/the-fairphone-gen-6
- iFixit laptop repairability scores: https://www.ifixit.com/repairability/laptop-repairability-scores
- Framework Laptop 16 repairability snapshot: https://www.ifixit.com/Document/ZdWVvQ2AHp5MDYgO/Repairability-Snapshot---Framework-Laptop-16.pdf

These sources provide comparison points, not independent proof that every model passes the same test. We must use matched test profiles before claiming superiority.

## 3. Rugged smartphone concept

### Mechanical architecture

1. **Flat, non-curved display.** Avoid edge glass and a fragile rear-glass panel. Use a structural mid-frame that prevents the display from becoming the chassis' primary bending member.
2. **Raised, replaceable impact perimeter.** Replaceable elastomer corner guards and a bezel stand slightly proud of the display so corner/face impacts are taken by the frame first.
3. **Sacrificial front layer.** Use a replaceable screen protector or mechanically serviceable outer lens separate from the expensive display module where optical, touch and sealing design permit. Publish the consumable's price and replacement procedure. A protector is not a guarantee against every impact.
4. **Display cassette.** A damaged panel should be replaceable without replacing the motherboard, cameras, frame or battery. Prefer screws and accessible connectors to deep adhesive stackups; validate drop sealing before and after repair.
5. **Shock path and isolation.** Use compliant supports and controlled clearances around the display and boards; avoid transferring corner impact directly into brittle glass, connector joints or battery edges.
6. **Sealed but serviceable enclosure.** Gaskets and replaceable port modules are preferred to permanent potting. Every service action that disturbs a seal must have a documented reseal test. USB-C protection should not make charging or repair unnecessarily difficult.
7. **Replaceable battery and port assembly.** Design for safe replacement with a small, ordinary tool set where feasible, while validating the water/dust seal and battery retention. A removable battery is a design trade-off, not a free feature.
8. **Repair-first materials.** No brittle glass back. Use replaceable bumpers and a replaceable rear cover; label screws, publish teardown/service documentation and stock key spares.

### Initial target envelope (hypotheses, not promises)

| Area | Prototype target | Qualification evidence |
|---|---|---|
| Drop survival | No cracked display, exposed sharp fragments, battery damage or loss of core function across a declared 1.5 m multi-orientation drop sequence onto a specified hard surface | Publish surface, orientations, sequence, units, preconditioning and every failure; expand to higher severity after first design passes |
| Repair after impact | Outer protector/lens and display cassette repair in <=10 minutes with common tools; no motherboard replacement for an isolated display fault | Timed repair trials by technicians who did not design the device |
| Screen resilience | Repeated free-fall/drop tests plus scratch/point-load and torsion tests | Run separately: scratch resistance is not impact resistance |
| Ingress | Target IP68 only if the enclosure passes the relevant IEC 60529 tests after both baseline manufacture and repeated field-repair cycles | Independent lab report; never infer an IP rating from gasket design alone |
| Service life | Publish spare-parts availability and software-support commitment for >=8 years, subject to supplier feasibility | Parts catalogue, supplier commitments, signed firmware release policy |
| Lifecycle | Minimize repair cost and total downtime, not merely warranty claim rate | Track cost per successful operating hour and mean time to restore service |

The exact test sequence, sample count and environmental limits are to be set after intended-user interviews and first prototypes. A single sample surviving a drop is not a reliability estimate.

### Critical design trade-offs

- **Gorilla-style strengthened glass / sapphire / polymer lens:** compare fracture energy, scratch resistance, optical quality, touch response, repairability and whole-device cost. Hardness is not the same as impact toughness; do not assume a harder cover alone solves breakage.
- **Removable battery vs. sealing:** compare an accessible gasketed hatch with a sealed screw-fastened battery cassette; test both with repaired seals.
- **Thinness vs. protection:** prioritize frame and shock travel before chasing flagship thinness.
- **OLED vs. LCD:** choose based on outdoor brightness, burn-in, low-temperature behavior, power and service cost—not fashion.
- **No universal invulnerability claim:** expose failure distributions and the exact conditions tested.

## 4. Serviceable field laptop concept

### Architecture

- 13–14 inch, non-glossy display with a replaceable sacrificial cover, raised bezel and replaceable display lid/cassette.
- Magnesium-alloy or equivalent structural frame; replaceable corner bumpers and protective port covers.
- IP65-class dust/water protection as a target, subject to airflow, cooling, keyboard and service-gasket validation. A fan-cooled laptop's vents and seals need careful co-design.
- Replaceable keyboard/top deck, hinges, battery, storage, memory where supported by the chosen platform, Wi-Fi module, and individual I/O daughterboards. Avoid soldered ports when a replaceable daughterboard is practical.
- Standardized fasteners, common tools, clear internal labels, public service manual, exploded drawings and part identifiers. Common failures should not force motherboard replacement.
- Optional dual batteries with a brief bridge-power period or true hot swap if the chosen electrical architecture supports it safely.
- Matte display with a target of 1,000 nits or better only if measured outdoor readability and battery/thermal cost justify it.
- USB-C Power Delivery plus an appropriately regulated vehicle/solar DC input, replaceable charging circuitry and multiple accessible ports.
- Linux/NixOS support contract: reproducible system configuration, offline recovery image, signed updates, rollback, local diagnostics and no cloud requirement to restore a working workstation.
- Data resilience: removable NVMe storage, documented encrypted backup and recovery, spare storage image, and explicit secure erase for replaced drives.
- Thermal design that is serviceable in dusty locations: accessible, replaceable filters or fans where needed and a documented cleaning interval.

### Initial target envelope

| Area | Prototype target | Evidence |
|---|---|---|
| Drop and shock | Start with the actual intended field profile; target no loss of core function after a specified 1.2 m multi-orientation drop sequence | Run with display open/closed and with/without power, then inspect connectors, hinge, battery and storage |
| Ingress | IP65 target for a fully assembled configuration, including keyboard and port covers | Independent IEC 60529 test; retest after specified repair cycles |
| Display | No display fracture in the declared drop sequence; screen cassette replaceable in <=10 minutes | Timed service trials and recorded failure analysis |
| Common repairs | Battery, SSD, keyboard and I/O daughterboard serviceable in <=10 minutes per module with ordinary tools | Timed trials; publish required tools and part prices |
| Parts | Key field-replaceable units available >=8 years; target each common module costs <25% of initial system MSRP where supplier economics allow | Binding supplier/repair program and public catalogue |
| Outdoor use | Readable in direct sunlight, glove-friendly controls, spill/dust tolerance and operating-temperature range defined from customer environments | Instrumented luminance, glove and environmental trials |
| Compute | Meet a frozen Symthaea/HDC workload and normal office/engineering workload at sustainable thermals | Exact source SHA, hardware ID, sustained latency/throughput, joules where measured and battery endurance |

The laptop drop height and IP class are initial study targets, not claims of equivalence to a certified rugged product. They must be revised with a service-life environmental profile before design freeze.

## 5. Qualification method

MIL-STD-810H is not a single pass/fail badge or a universal list of required design limits. The US Defense Logistics Agency describes it as an environmental tailoring process that derives realistic tests from a system's life-cycle requirements. Choose applicable methods and severities for the intended user; publish them.

- DLA standard details: https://quicksearch.dla.mil/qaDocDetails.aspx?ident_number=35978
- IEC 60529 IP-code publication: https://webstore.iec.ch/en/publication/2447

Test plan should include:
1. Multi-orientation drop sequence onto a specified surface, with mass/configuration recorded.
2. Repeated drops, torsion/bending and corner compression.
3. Dust/water ingress before and after repair cycles.
4. Hot/cold operating and storage exposure, thermal cycling and condensation assessment.
5. Vibration/shock consistent with backpack, vehicle and workshop transport.
6. Port insertion/removal cycles, hinge and button cycles, keyboard spill/contamination.
7. Battery retention and safety inspection after each mechanical sequence.
8. Sunlight readability, thermal throttling, radio performance and battery endurance under load.
9. Post-test functional checks, seal inspection, fault logging and teardown of a declared subset.
10. Repair trials with timed operations, standard tools, replacement-part cost and a second ingress test after reassembly.

Pre-register criteria. Publish device count, lot/build version, fixture calibration, videos or raw sensor data where practical, all failures and any exclusions. Run baseline and stressed devices through the same functional suite.

## 6. Benchmark that could beat industry practice

A multi-axis **Field Reliability and Recovery Benchmark** should combine the strengths commonly split across rugged-device and repairability reviews:

- damage/failure probability by event and orientation;
- successful operating hours, mission availability and unplanned downtime;
- mean time to diagnose and mean time to repair;
- repair cost, parts availability and percent of failures fixed without motherboard replacement;
- weight/volume, battery life, sustainable performance and thermal recovery;
- ingress performance before and after servicing;
- firmware/OS recovery without network access;
- lifecycle cost per successful operating hour.

A weighted single score risks hiding trade-offs. Publish the full Pareto frontier and raw results. Compare at similar weight, price, compute class and use profile. A very thick, expensive device that survives a severe drop does not automatically beat a lighter unit whose damage is cheaper to fix.

For compute/energy comparisons, follow fixed-workload discipline rather than borrowing an official score: MLPerf Tiny describes defined tasks, quality targets, latency/energy rules and calibrated power measurement. Reference: https://mlcommons.org/working-groups/benchmarks/tiny/

## 7. Prototype plan — no custom silicon yet

**P0 — Field requirements:** interview tradespeople, utility crews, farmers, emergency responders and rural users; record actual drops, dust/water, heat, glove use, charging, repair access and service delays. Define lifecycle environmental profiles.

**P1 — Phone front-stack experiment:** build several cheap mechanical mockups around an off-the-shelf display/compute board. Compare raised bezel + protector, separate replaceable lens, corner bumper geometry and screen mounting. Run instrumented drop tests; stop any configuration that transfers impact into the display.

**P2 — Laptop enclosure experiment:** start with an available repairable laptop compute platform and add a serviceable bumper/display protection concept. Measure weight, thermal impact and repair access before considering a custom chassis.

**P3 — Repair and ingress loop:** repeatedly service the most likely damaged modules, reassemble, retest seals and measure repair time. Document whether modularity has degraded ingress protection.

**P4 — Small qualification lot:** after a design is stable, run a declared sample lot with pre-registered drop/environment sequences. Use enough samples to make the statistical claim honest; a few engineering prototypes are for design iteration, not certification.

**P5 — Product decision:** commission a custom mainboard only if test data show a meaningful gap that available modules cannot meet. Obtain supplier quotes, repair/spares commitments and independent laboratory pricing before promising a release date or retail price.

## 8. First decision

Proceed with the **field laptop** as the first platform prototype: the Linux/NixOS recovery and modular-repair story is immediately testable with existing boards and parts. In parallel, build a **phone display-protection experiment**, not a phone from scratch. The phone's radio, camera, antenna, thermal and battery integration risks make a full custom device a much larger capital commitment.

The defensible customer promise is not “a screen can never break.” It is: *the screen is protected by a replaceable sacrificial system, the device is tested under published conditions, and damage can be repaired quickly with affordable, stocked parts.*

## 9. Evidence boundary

This document records architecture hypotheses and a verification plan. It does not describe a built prototype, laboratory result, certification, IP rating or commercial cost estimate. All target values are provisional until user requirements, suppliers and tests constrain them.
