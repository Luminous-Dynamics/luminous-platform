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
| IEC 61439-4:2023 | Particular requirements for low-voltage assemblies for construction sites, including transportable (semi-fixed) or mobile enclosed assemblies; up to 1,000 V AC / 1,500 V DC | PF-FIELD-MOBILE construction-site assemblies and relevant temporary/site distribution products | Applies to the assembly boundary, not merely the converter module inside it |
| IEC 60364-7-704:2017 | Requirements for electrical installations at construction and demolition sites; scope includes fixed or moveable installations | Site installation, connection, distribution and temporary construction-site wiring | Installation requirements are distinct from equipment assembly product requirements |
| IEC 62933-5-1:2024 | Safety considerations for grid-integrated electrical energy storage systems | Later integrated BESS / grid-connected energy storage system architecture | Use only where the storage system and grid-integration context fit |
| IEC 62933-5-4:2026 | Representative safety test methods and procedures specifically for grid-integrated EES using lithium-ion battery subsystems; published 2026-05-11 | Battery subsystem test-method research if a grid-integrated lithium-ion ESS is later selected | IEC describes representative methods and says the publication does not include all test methods; not a universal portable-power certificate |
| IEC 62619:2022 | Safety requirements for secondary lithium cells and batteries for use in industrial applications | Selected industrial lithium battery subsystem and battery pack supplier qualification | Battery-cell/battery-system scope must be resolved with product standards and system-level ESS safety |
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

## 3. Important scope exclusion: HVDC breaker standard is not a 48 V component standard

IEC TS 62271-313:2025 covers DC circuit-breakers for indoor or outdoor use with direct voltages of **100 kV and above**, for DC transmission and distribution systems. It includes mechanical switching devices, power-electronic switches, auxiliary circuits, energy dissipation and controls; unidirectional and bidirectional operation are possible depending on design.

This is useful future context for very-high-voltage DC transmission protection, but it is **not** the applicable product standard for a 48 V SSPC, a 400/800 VDC field product or a low-voltage data-centre bus.

Source: https://webstore.iec.ch/en/publication/64996

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
