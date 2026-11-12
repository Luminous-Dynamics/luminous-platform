# Power Fabric standards applicability and qualification map

**Snapshot:** 2026-11-12  
**Status:** public-source research / applicability map only; this is not a legal determination, standards interpretation, conformity assessment or certification plan approved by a notified/accredited body.

## 1. Why the map is configuration-specific

Power Fabric spans several product types that do not share one certification route:

- Low-voltage isolated DC/DC laboratory building blocks.
- DC branch power controllers and possible circuit-breaker / isolator products.
- AC/DC conversion and inverter equipment.
- Transportable construction-site assemblies.
- Battery-integrated portable / mobile energy storage.
- Industrial plant temporary supply assemblies.
- Later medium-voltage or HVDC conversion and protection.

A standard is relevant only when its scope and the actual product / installation / intended user fit. A reference to a standard is not evidence of compliance. Each product configuration needs a standards applicability decision, licensed-text review, test mapping and conformity route with qualified specialists.

## 2. Standards with direct relevance to current work

| Standard | Published scope / current research observation | Power Fabric relevance | Claim boundary |
|---|---|---|---|
| IEC 62477-1:2022 | General safety requirements for power electronic converter systems and accessories, including control/protection/monitoring, rated up to 1,000 V AC or 1,500 V DC | PEBB-LV-01, DC/DC converters, later 800 VDC conversion and relevant converter accessories | Safety reference for PECS; does not itself qualify a complete installation, breaker or field assembly |
| IEC 60947-2:2024, Ed. 6 | Low-voltage circuit-breakers up to 1,000 V AC / 1,500 V DC; 2024 revision includes suitability-for-isolation changes, dielectric testing in tripped position and DC dielectric test changes | Mandatory study if a Luminous product is intended to be claimed as a circuit-breaker or isolation-capable breaker | An eFuse / SSPC does not become a circuit-breaker by using a MOSFET; full product construction and prescribed tests matter |
| IEC 61439-4:2023 | International IEC edition: particular requirements for low-voltage assemblies for construction sites, including transportable (semi-fixed) or mobile enclosed assemblies; up to 1,000 V AC / 1,500 V DC | PF-FIELD-MOBILE construction-site assemblies and relevant temporary/site distribution products | Applies to the assembly boundary, not merely the converter module inside it; do not confuse with regional EN/SANS adoptions |
| EN IEC 61439-4:2026 (CENELEC) | Regional European adoption of the IEC 61439-4:2023 technical text, published 2026-10-29; national implementation date 2027-10-31 in the cited CENELEC foreword | Important current European market update for construction-site assemblies; monitor for products intended for EU markets | This is a European regional adoption, not a new international IEC edition and not automatically the applicable South African SANS edition. Confirm the national adoption and transition dates in each target market |
| IEC 60364-7-704:2017 | Requirements for electrical installations at construction and demolition sites; scope includes fixed or moveable installations | Site installation, connection, distribution and temporary construction-site wiring | Installation requirements are distinct from equipment assembly product requirements |
| IEC 62933-5-1:2024 | Safety considerations for grid-integrated electrical energy storage systems | Later integrated BESS / grid-connected energy storage system architecture | Use only where the storage system and grid-integration context fit |
| IEC 62933-5-4:2026 | Representative safety test methods and procedures specifically for grid-integrated EES using lithium-ion battery subsystems; published 2026-05-11 | Battery subsystem test-method research if a grid-integrated lithium-ion ESS is later selected | IEC describes representative methods and says the publication does not include all test methods; not a universal portable-power certificate |
| IEC 62619:2022 | Safety requirements for secondary lithium cells and batteries for use in industrial applications | Selected industrial lithium battery subsystem and battery pack supplier qualification | Battery-cell/battery-system scope must be resolved with product standards and system-level ESS safety |
| IEC 62933-2-1:2017 | General EES unit parameters and testing methods | Establish storage capability definitions and test measures | Generic system parameters; application-specific duty cycles are handled by Part 2-2 |
| IEC TS 62933-2-2:2022 | Application and performance testing with EES duty cycles and test methods | Matched generator/BESS mission profiles and storage performance comparisons | Does not replace battery safety, converter safety or system installation assessment |
| IEC TS 62933-2-3:2025 | EES performance assessment testing during site operation after commissioning | Post-commissioning verification and field performance follow-up | Site-performance test methods, not a product safety certification |
| ISO 8528-5:2025, Edition 6 | Design and performance criteria for reciprocating-engine-driven AC generator sets operating as a unit | Generator-set subsystem and its declared source response in hybrid mission profiles | Applies to the RIC-engine/AC-generator set; does not qualify the complete hybrid power system |
| IEEE 2030.3-2016 | Active test procedures for electric energy storage equipment and systems for electric power systems applications | Supplementary storage test-procedure reference where the system's application fits | Scope is electric power system storage equipment; not automatically a portable product certificate |
| IEEE 1679-2020 | Recommended practice for characterization/evaluation of stationary storage technologies, including conditions and comparison limits | Storage technology characterization and transparent comparator conditions | Stationary storage focus; do not assume every mobile/portable use is covered |
| IEC 60529 | Degrees of protection provided by enclosures (IP Code) | Enclosure ingress protection selection and verification for dust/water exposure | An IP label does not establish vibration, corrosion, temperature, UV, thermal performance or overall field suitability |
| IEC 60068-2-64:2026, Ed. 3 | Environmental Test Fh: broadband random vibration; published 2026-10-08, replaces the 2008 + AMD1:2019 edition | Transported/mobile equipment, enclosures, connectors, busbars, PCBs, battery mounting and post-vibration function | Test severity and mounting must come from the declared mission profile; one vibration test does not qualify all transport environments |
| IEC 60204-1:2016 + AMD1:2021 | Electrical equipment of machines, for machines not portable by hand while working | Selected industrial machine integration, including where a power system is part of a coordinated machine system | Not a generic portable power-station product standard; applicability depends on machine boundary |

Primary publisher references:
- IEC 62477-1:2022: https://webstore.iec.ch/en/publication/28936
- IEC 60947-2:2024: https://webstore.iec.ch/en/publication/66277
- IEC 61439-4:2023: https://webstore.iec.ch/en/publication/69748
- IEC 60364-7-704:2017: https://webstore.iec.ch/en/publication/33784
- IEC 62933-5-4:2026: https://webstore.iec.ch/en/publication/67442
- IEC 60068-2-64:2026: https://webstore.iec.ch/en/publication/104354
- IEC 60204-1:2016 + AMD1:2021: https://webstore.iec.ch/en/publication/71256

## 3A. October/November 2026 regional update: EN IEC 61439-4:2026

CENELEC approved EN IEC 61439-4:2026 as a regional European standard for construction-site assemblies. The cited European foreword gives a latest national implementation date of 2027-10-31 and a latest withdrawal date for conflicting national standards of 2029-10-31. The NBN catalogue records its publication on 2026-11-01 and replacement of EN 61439-4:2013.

This does **not** mean IEC published a new international IEC 61439-4 edition in October 2026. The IEC Webstore page still identifies IEC 61439-4:2023 as its base publication. The CENELEC text is a regional adoption of that international edition. In addition, the cited scope material distinguishes the complete construction-site assembly from individual self-contained components and devices, which remain subject to their respective product standards.

For Luminous:
- Maintain IEC 61439-4:2023 as the international assembly reference unless the IEC publisher reports a new edition.
- Add EN IEC 61439-4:2026 to the EU market-access checklist and monitor national transition/adoption.
- Do not apply the regional EN adoption automatically to a South African product.
- Ask NRCS / the relevant SANS standards owner which national edition and compulsory-specification status apply to the exact equipment category and intended market.

Sources:
- IEC base publication: https://webstore.iec.ch/en/publication/69748
- CENELEC foreword / transition dates: https://standards.iteh.ai/catalog/standards/clc/08941488-da48-4734-9a5a-582a1670abff/en-iec-61439-4-2026
- Belgian national standards catalogue publication record: https://app.nbn.be/data/r/platform/frontend/detail?p40_id=3597438

## 3. Important scope exclusion: HVDC breaker standard is not a 48 V component standard

IEC TS 62271-313:2025 covers DC circuit-breakers for indoor or outdoor use with direct voltages of **100 kV and above**, for DC transmission and distribution systems. It includes mechanical switching devices, power-electronic switches, auxiliary circuits, energy dissipation and controls; unidirectional and bidirectional operation are possible depending on design.

This is useful future context for very-high-voltage DC transmission protection, but it is **not** the applicable product standard for a 48 V SSPC, a 400/800 VDC field product or a low-voltage data-centre bus.

Source: https://webstore.iec.ch/en/publication/64996

## 3B. Grid-forming interoperability is a separate performance qualification

**IEEE 2800-2022** is an active transmission/sub-transmission interconnection and performance standard for inverter-based resources. It addresses ride-through, active/reactive power control, dynamic support, power quality and protection.

**IEEE 2800.2-2026** is now a published, active IEEE recommended practice (published 2026-06-30). It provides plant-level conformity-assessment practices for demonstrating conformity with IEEE 2800, including equipment, conditions, tests, modelling methods and verification procedures. Its scope is transmission/sub-transmission-connected IBR plants; it is not a product certificate for a portable field power unit.

Neither IEEE 2800-2022 nor 2800.2-2026, by itself, proves every GFM-specific behavior. A 2026 review notes that IEEE 2800 conformity does not necessarily demonstrate phase-angle-jump response, GFM-specific inertia behavior, autonomous island formation, GFM/GFL mode switching or black start. Those require supplementary GFM-specific or project-specific criteria.

A sound GFM test programme composes RMS/EMT simulation, real-time digital simulation, controller-HIL, power-HIL, laboratory tests and field demonstrations. It must address model fidelity, current/energy limits, multi-vendor interaction and harmonized acceptance criteria.

Power Fabric's grid-interactive product roadmap should therefore maintain separate evidence claims for:
- grid-following interconnection performance under applicable connection requirements;
- plant-level conformity evidence using IEEE 2800 / 2800.2 where that transmission-level scope actually applies;
- grid-forming voltage-source behavior and weak-grid operation;
- islanding and resynchronization;
- black-start and staged restoration;
- fault ride-through and protection coordination;
- multi-vendor interoperability and communication loss.

Do not use an IEEE 2800 / 2800.2 reference as a blanket statement that a portable unit is grid-forming or black-start capable.

Sources:
- IEEE 2800-2022 official scope: https://standards.ieee.org/ieee/7003/10453/
- IEEE 2800.2-2026 official publication and scope: https://standards.ieee.org/ieee/2800.2/10616/
- 2026 GFM testing review: https://www.mdpi.com/1996-1073/19/20/4966

## 3C. Transient event and voltage-dip immunity references

The mission/event model uses separate source event capability, storage event capability and common
output-path capability. This distinction prevents a static "surge" number or an arithmetic sum of
source and battery ratings from being mistaken for delivered motor-start or ride-through service.

| Standard | Current published snapshot | Relevance | Boundary |
|---|---|---|---|
| IEC 61000-4-11:2020, Ed. 3 | Voltage-dip, short-interruption and voltage-variation immunity methods for equipment with rated input current up to 16 A per phase on 50/60 Hz LV AC networks | Immunity of relevant connected equipment to defined supply disturbances | Does not certify generator load acceptance, motor start, source ride-through or the output capability of a hybrid power plant |
| IEC 61000-4-34:2005+AMD1:2009+AMD2:2025, consolidated Ed. 1.2 | Same disturbance class for equipment with mains current greater than 16 A per phase; the 2025 amendment updates the term from input current to mains current | Immunity test method for higher-current equipment | Not a generator/motor-start capability standard |
| IEC 61000-4-4:2012, Ed. 3 | Electrical fast transient/burst immunity for supply, signal, control and earth ports | EMC immunity for relevant electronics and equipment | Does not establish power-stage transient output capability |
| ISO 8528-5:2025, Ed. 6 | Performance criteria for an RIC engine and AC generator operating as a generating set | Generator-set source subsystem load acceptance/dynamic performance programme | Does not qualify the whole hybrid system or battery support |
| IEC TS 62933-2-2:2022 | Application-specific EES duty cycles and performance testing | Storage duty-cycle and energy performance programme | Not a complete transient waveform or battery safety qualification |

Publisher sources:
- IEC 61000-4-11:2020: https://webstore.iec.ch/en/publication/63503
- IEC 61000-4-34 consolidated 2025: https://webstore.iec.ch/en/publication/109082
- IEC 61000-4-4:2012: https://webstore.iec.ch/en/publication/4222
- ISO 8528-5:2025: https://www.iso.org/standard/85960.html
- IEC TS 62933-2-2:2022: https://webstore.iec.ch/en/publication/64570

The IEC 61000 methods above are equipment immunity tests against specified disturbances. They are not direct motor-start or generator transient performance certificates. Product boundary, current class, installation and jurisdiction determine applicability.

## 4. Competitive protection benchmark and its implications

ABB states that SACE Infinitus is a solid-state circuit breaker with IEC 60947-2 certification, combining semiconductor interruption and electromechanical isolation. ABB publishes figures up to 2,500 A / 1,250 V DC and interruption times down to 15 microseconds for the platform. This is a mature high-current commercial benchmark, not a fair same-class comparator for a 48 V auxiliary branch controller.

Source: https://www.abb.com/global/en/areas/electrification/low-voltage/circuit-breakers/solid-state-circuit-breakers/sace-infinitus

TI's TIDA-050090 is another relevant comparison but is a 54 V / 5 kW eFuse reference design, not a breaker certification. It uses one TPS1689 and five TPS1685 parts in parallel; TI reports 100 A RMS at 55 °C without external airflow and says the fully assembled board is for testing/performance validation, not for sale.

Source: https://www.ti.com/tool/TIDA-050090

The comparison must keep these product classes distinct:
- eFuse / hot-swap IC.
- Board-level power-path protection reference design.
- Semiconductor branch power controller.
- Circuit-breaker assembly.
- Isolation device.
- Hybrid semiconductor/mechanical protection system.

Do not transfer an ABB breaker claim or TI reference-board thermal result to a Luminous design.

## 5. South African conformity and legal research workstream

The National Regulator for Compulsory Specifications (NRCS) explains that compulsory specifications are technical regulations requiring conformity to health, safety or environmental requirements; the standard made compulsory must be a national (SANS) standard, or another standard/specification may be made mandatory where a suitable national standard does not exist.

Therefore, the programme should not assume that every imported module or complete field power unit automatically follows the same Letter of Authority or compulsory-specification path. For each sellable product and included item, determine:
1. Exact product classification and intended use.
2. Whether a current compulsory specification / VC applies to that product category.
3. Relevant current SANS adoption and edition.
4. Whether NRCS approval, LOA, test report or other market-access evidence is required.
5. Whether the product is sold as a component, complete assembly, battery system, inverter, generator hybrid package or site installation.
6. Importer/manufacturer obligations, marking, instructions, local testing and surveillance duties.
7. Site-specific electrical installation, construction safety, plant-owner and utility/distributor approvals.

NRCS public overview: https://www.nrcs.org.za/compulsory-specifications/about-compulsory-specifications/

This map does not establish the present compulsory-specification schedule or determine whether a particular Luminous product needs an LOA. Confirm the answer directly with NRCS and a South African conformity-assessment specialist before procurement for resale, importation or deployment.

## 6. Product-family applicability matrix

| Family | First standards to screen | Additional work |
|---|---|---|
| PEBB-LV-01 isolated 36–60 V DC/DC | IEC 62477-1; component-level insulation, EMC and thermal requirements derived from use | Qualified design review; appropriate dielectric / insulation test plan; no field or mains claim |
| 48 V DC branch controller / SSPC | IEC 62477-1 for converter/switching equipment; IEC 60947-2 only if the finished product is within the circuit-breaker scope and claims that function | Fault-current and I²t, isolation suitability, DC dielectric, EMC, fail-short and backup-protection coordination |
| PF-FIELD-PORTABLE | IEC 62477-1 for relevant converter elements; IEC 60529 and IEC 60068 environmental methods | Enclosure / transport / battery / wiring / connector requirements determined from product boundary |
| PF-FIELD-MOBILE construction-site assembly | IEC 61439-4 and IEC 60364-7-704; relevant converter and battery standards | Assembly verification, earthing, protective devices, temporary wiring, mechanical transport, commissioning and site rules |
| PF-FIELD-PLANT | Relevant assembly, converter, battery and installation standards | Site engineering / electrical authority approval, permit-to-work, protection coordination and plant-specific safety requirements |
| Integrated grid-connected BESS | IEC 62933-5-1 and applicable battery-cell/system standards; IEC 62933-5-4:2026 where applicable | Grid-code, utility/distributor, fire safety, installation, protection and accredited system-level test programme |
| Later HVDC / MV equipment | Product-specific medium/high-voltage and converter standards | Consortium, accredited high-energy laboratories and specialist protection engineering; IEC TS 62271-313 is only for DC systems >=100 kV |

## 7. Release gates

A product cannot move from engineering prototype to field pilot merely because all software tests pass. Minimum progression:

1. **Applicability gate:** exact product boundary and relevant jurisdiction documented by a competent reviewer.
2. **Design gate:** hazard analysis, insulation/earthing, fault-energy and protection architecture reviewed.
3. **Model gate:** model assumptions, correlation and uncertainty disclosed.
4. **Bench gate:** safe, controlled, calibrated physical tests on an exact configuration.
5. **Environmental gate:** product-specific ingress, temperature, vibration, transport, EMC and post-stress checks.
6. **Assembly gate:** prescribed design verification / routine verification and installation documentation.
7. **Independent gate:** required accredited test-lab and conformity steps complete.
8. **Field gate:** site-owner / utility / plant approval where applicable, controlled installation and operating evidence.

Every gate has an owner, evidence reference, configuration, acceptance criterion and expiry/requalification rule.

## 8. Update protocol

This document is a dated snapshot. Before each product release or standards review:
- Check publisher pages for current editions and corrigenda.
- Check local SANS adoptions and compulsory-specification schedules.
- Record date checked and reviewer.
- Treat withdrawn or superseded editions explicitly.
- Pin exact standard revision in the product compliance matrix.
- Do not quote purchased standard clause text in public repositories.


## 9. Site-operational EES assessment and evidence records

IEC TS 62933-2-3:2025 is directly relevant to post-commissioning site performance assessment. The
IEC publisher describes it as unit parameters and test methods for validating/assessing EES
performance after commissioning. Its public preview distinguishes this operational-data assessment
from precise laboratory testing and identifies representative sampling, preprocessing of missing
values/anomalies/noise, and operational records such as power, voltage, current, energy, faults,
maintenance and dispatch logs. It also enumerates performance items including actual storage
capacity, input/output power, round-trip efficiency, auxiliary consumption, available hours,
equivalent operation factor and availability/outage indicators.

For Power Fabric this suggests two complementary evidence classes:
- controlled tests for precise characterization and matched component/system comparisons;
- site-operational assessments for lifecycle utilization, degradation, outages, availability,
  auxiliary consumption and real-world duty-cycle performance.

The site passport must preserve the primary POC and any separate auxiliary POC, sample selection,
time-series processing, data completeness, fault/maintenance/dispatch histories and exact system
configuration. The public sample preview recommends at least three representative samples as a
general recommendation; this programme uses a declared minimum (at least three for site-operation
runs), not as a claim of IEC conformity. Confirm detailed requirements in the licensed standard
and product applicability before any compliance claim.

Sources:
- IEC TS 62933-2-3:2025 publisher page: https://webstore.iec.ch/en/publication/77334
- IEC TS 62933-2-3:2025 public preview: https://cdn.standards.iteh.ai/samples/iec/iec-ts-62933-2-3-2025/8df418d2566940208430adcf448df1ab/iec-ts-62933-2-3-2025.pdf

## 10. Microgrid monitoring, control and energy-management system testing

[IEC TS 62898-3-5:2026](https://webstore.iec.ch/en/publication/84175), published 11 June 2026, provides requirements for hardware-in-the-loop testing, commissioning testing and periodic testing of microgrid monitoring, control and energy-management systems (MMCS/MEMS), for grid-connected and isolated microgrids.

This is directly relevant to the Power Fabric controller test ladder. Use it alongside:
- IEEE 2030.7-2017 for the common functional specification of microgrid controllers;
- IEEE 2030.8-2018 for controller testing procedures and performance comparison metrics;
- IEC TS 62898-3-5:2026 for HIL, commissioning and periodic testing requirements.

The IEEE revision projects P2030.7 and P2030.8 are active PARs in this snapshot, not published replacement editions. Confirm all detailed requirements in the licensed standards and with qualified test laboratories. This applicability map is not a conformity assessment.

Source: https://webstore.iec.ch/en/publication/84175


## 11. Measurement uncertainty and nonlinear models — 2026 update

The BIPM/JCGM catalogue now lists [JCGM 100:2008/Amd.1:2026](https://www.bipm.org/en/web/guest/publications/guides), an amendment addressing nonlinearity in measurement models, and [JCGM GUM-5:2026](https://www.bipm.org/en/committees/jc/jcgm/publications), alongside:
- JCGM 101:2008 — propagation of distributions using Monte Carlo;
- JCGM 102:2011 — multiple output quantities;
- JCGM 106:2012 — role of measurement uncertainty in conformity assessment;
- JCGM GUM-6:2020 — developing and using measurement models.

Power Fabric's passport/comparator PR #430 stores declared absolute uncertainty bounds and uses a conservative sum for pairwise diagnostic deltas. That is a deliberately limited screen, not a complete uncertainty budget or proof of statistical significance. It must not silently imply that inputs are independent, linear, Gaussian or free from common-reference correlation.

Mycelix owns the measurement model, covariance/correlation, nonlinear propagation, calibration applicability, uncertainty receipt and invalidation semantics (notably issues #6034, #6042 and #6048). Platform binds and consumes the exact model/result receipt. Where nonlinearity is material, the uncertainty owner should justify an applicable higher-order or distribution-propagation method and bind the implementation, input distributions, correlation model, random algorithm/seed where relevant, convergence and coverage semantics.

Sources:
- BIPM Guides in Metrology: https://www.bipm.org/en/web/guest/publications/guides
- JCGM publications: https://www.bipm.org/en/committees/jc/jcgm/publications
- ISO/IEC Guide 98-3:2008/Amd.1:2026: https://www.iso.org/standard/92716.html

This standards/method crosswalk is not a claim that Power Fabric performs accredited uncertainty evaluation or conformity assessment.
