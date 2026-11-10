# Sources and competitive baseline

Research snapshot: **2026-11-10**. Links are primary publisher, standards-body, laboratory,
manufacturer or South African utility sources where available. This list is a starting evidence
corpus, not a complete standards review.

## Solid-state transformer and modular DC/DC

1. **IEEE Open Journal of Power Electronics**, “A Review of DC-DC Converters for Modular Solid-State Transformers: Challenges, Comparative Analysis, and Design,” Volume 7, pp. 1846–1864, published 3 June 2026. DOI: [10.1109/OJPEL.2026.3699737](https://doi.org/10.1109/OJPEL.2026.3699737). Reports experimental validation of three PEBB topology options and 98.55% peak efficiency in its case study. The figure is a literature anchor, not an equivalent-performance guarantee for PEBB-LV-01.
2. **IEEE / ScienceDirect**, “Solid state transformer in energy conversion and distribution: a structured design framework, four-dimensional taxonomy, control strategies, and deployment roadmap,” 2026. [ScienceDirect](https://www.sciencedirect.com/science/article/pii/S259017452600437X). Useful for SST system taxonomy, topology-selection methodology, maturity and deployment barriers.
3. **IEEE**, “Qualification Challenges of Solid-State Transformers: Standards, Medium Voltage Power Semiconductors, and Auxiliary Circuits,” IPEC-Nagasaki 2026. [IEEE Xplore](https://ieeexplore.ieee.org/abstract/document/11597371). Highlights medium-voltage semiconductor, medium-frequency transformer and auxiliary-circuit qualification as deployment barriers.
4. **IEEE**, “Insulation Challenges in Medium-Voltage High-Frequency Transformers for Solid-State Transformers: A Review of Mechanisms, Diagnostics, and Mitigation,” early access, 18 August 2026. [IEEE Xplore](https://ieeexplore.ieee.org/abstract/document/11658780). Covers coupled dv/dt, frequency, temperature and insulation degradation / partial discharge issues.

## Semiconductor and component reliability

5. “Reliability Challenges, Models, and Physics of Silicon Carbide and Gallium Nitride Power Devices,” Energies, 18(5), 1046, 21 February 2025. [Open access article](https://www.mdpi.com/1996-1073/18/5/1046). Reviews thermal degradation, defect propagation, charge trapping and robustness testing.
6. “Mission-profile based reliability framework for medium-frequency transformers,” Power Electronic Devices and Components, Volume 12, December 2025, 100114. [DOI 10.1016/j.pedc.2025.100114](https://doi.org/10.1016/j.pedc.2025.100114). Uses mission-profile stress, insulation wear-out, partial-discharge-related effects and Monte Carlo uncertainty in a 170 kW, 15 kHz DAB case study.

## Solid-state protection / commercial competitor

7. **ABB**, [SACE Infinitus — Solid-State Circuit Breaker](https://www.abb.com/global/en/areas/electrification/low-voltage/circuit-breakers/solid-state-circuit-breakers/sace-infinitus). Manufacturer-reported platform ratings: up to 2,500 A / 1,250 V DC, interruption down to 25 μs, IEC 60947-2 certification, solid-state interruption with electromechanical isolation. Record vendor claims as vendor claims; independently test any competing product.
8. **ABB**, [SACE Infinitus product overview](https://new.abb.com/low-voltage/products/circuit-breakers/sace-infinitus-solid-state-circuit-breaker). Additional product architecture and application descriptions.

## Grid-forming and hardware-in-the-loop

9. **Energy Systems Integration Group**, “Testing the Performance of Grid-Forming Resources: Test Methods and Performance Metrics for Evaluating the Voltage Source Behavior of Grid-Forming Resources,” September 2025. [Report page](https://www.esig.energy/reports-briefs/testing-the-performance-of-grid-forming-resources/). Important future microgrid benchmark source.
10. **National Laboratory of the Rockies**, [Grid Simulation and Power Hardware-in-the-Loop](https://www.nlr.gov/grid/simulation-phil). Describes 1.08 MVA and 7 MVA grid simulators and real-time PHIL capabilities.
11. **National Laboratory of the Rockies**, “Performance Evaluation of a Single-Phase Grid-Forming Inverter Through Hardware Experiments,” 2024. [Publication record](https://research-hub.nlr.gov/en/publications/performance-evaluation-of-a-single-phase-grid-forming-inverter-th-2/). Demonstrates why transition transients and frequency-support behavior must be measured, not assumed.
12. **IEEE Standards Association**, [P2800.1 project](https://standards.ieee.org/ieee/2800.1/12387/). Project for recommended practice on capabilities and performance of grid-forming IBR/CBR equipment; the cited page identifies it as an active project, not a published final standard.

## Standards and South African context

13. **IEC**, [IEC 62909-1:2025 — Bi-directional grid-connected power converters, Part 1: General and safety requirements](https://webstore.iec.ch/en/publication/69114). Covers specified bidirectional grid-connected converters with AC/DC port voltage scope and includes safety and performance requirements. Its scope explicitly excludes certain PV, UPS and EV charging equipment; product-specific standards must be mapped separately.
14. **SSEG / NRS**, [NRS 097-2-1:2024 — Grid interconnection of embedded generation, Part 2, Section 1: Utility interface](https://www.sseg.org.za/wp-content/uploads/2024/02/NRS-097-2-1-Published-2024.pdf). South African small-scale embedded-generation utility-interface context.
15. **Eskom Distribution**, [Small-scale embedded generators](https://www.eskom.co.za/distribution/small-scale-embedded-generators/). Current registration and documentation information for relevant Eskom-connected SSEG installations.
16. **Eskom**, [EGI Report, March 2026](https://www.eskom.co.za/distribution/wp-content/uploads/2026/03/EGI-Report-March-2026_RP_240-153339608.pdf). Describes NRS 097-2-1 compliance-verification pathways including accredited third-party type-test evidence.

## Reading and interpretation cautions

- The 98.55% SST paper result is a peak result for a specific experimental topology and case study. It is not directly comparable to a different voltage, power, cooling, isolation and measurement boundary.
- ABB product values are manufacturer specifications, not independent head-to-head lab measurements.
- ESIG / NLR grid-forming work applies to later AC/microgrid stages, not the first DC/DC demonstrator.
- Standards must be purchased/read in full and applicability confirmed by a qualified compliance specialist. Abstracts, catalog pages and search-result snippets do not establish conformity.
- Recheck standard editions, local utility requirements and product-scope exclusions before any design freeze or certification plan.
