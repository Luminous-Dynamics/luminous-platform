# Industry baseline and matched-competition plan v0.1

**Snapshot:** 2026-11-10  
**Parent:** [Luminous Power Fabric #364](https://github.com/Luminous-Dynamics/luminous-platform/issues/364)  
**Purpose:** identify real commercial and experimental comparators before target claims are frozen.  
**Status:** desk research only; no Luminous-vs-competitor physical comparison has been performed.

## 1. Closest current commercial comparators

| Product / reference | Vendor- or paper-reported specification | What it is useful to benchmark | Why it is not a fully matched competitor |
|---|---|---|---|
| **Wang et al., bidirectional CLLC prototype (Electric Power Systems Research, Jun 2026)** | A 500 W laboratory prototype using FHA-based resonant design; reported ZVS and 97.6% peak forward-transfer efficiency | Closest same-rated-power research prototype for isolated bidirectional CLLC topology research | Abstract reports peak forward efficiency, not matched reverse-direction or full 36–60 V map; detailed operating-point conditions must be extracted from full paper before direct comparison |
| **ELSİS BIDIR Module** | 500 W bidirectional buck-boost; 9–60 V DC input; adjustable 3.3–48 V output; official product page states >97% and datasheet feature list states >97% at full rated load current; 1/8-brick, 25.90 × 61.72 mm, 66.2 g, I²C interface | Closest functional 500 W bidirectional low-voltage commercial benchmark; compactness, control interface, output range, full-load efficiency and protection feature baseline | Datasheet cover summarizes efficiency as >95%, while its feature list says >97% at full rated load current; request conditions/curves from vendor. Public specification does not state galvanic isolation; vendor claims are not independent matched measurements |
| **MPS MPC10106-54-0500** | 500 W continuous; 40–60 V input; 8:1 output ratio; non-isolated LLC; 2,634 W/in³ stated power density; 97.8% peak efficiency in the vendor's Q4 2024 48 V data-centre brochure | High-density 48 V power conversion, fixed-ratio module packaging, semiconductor/thermal integration and data-centre architecture | Not bidirectional, not isolated, and converts 48 V class to a low-voltage point-of-load bus; reported peak is not a full-map result |
| **MPS MPC10106-B-54-0500** | 500 W-class, 40–60 V input, 8:1 fixed-ratio non-isolated LLC; 3,019 W/in³ and 97.65% peak in MPS's 2025 brochure | Newer data-centre power-density and fixed-ratio module reference | Not bidirectional or isolated; SKU and brochure revision differ from the 2024 MPC10106-54-0500; not a matched benchmark |
| **Vicor V48A48C500 family** (e.g. V48A48C500BG) | 500 W isolated module; 36–75 V input; 48 V output; 3 kV isolation; 89.7% typical efficiency at nominal input / 75% load / 25 °C in the family datasheet | Adjacent commercial isolation benchmark, packaging, input range and insulation interface | Not bidirectional; family suffixes have different lifecycle states (DigiKey lists BG active and BL2 obsolete at snapshot); confirm exact current SKU and revision before procurement |
| **IEEE Access 25 kW dual DAB–CLLC** | Experimentally validated 25 kW dual-module system over 200–1000 V; 98.37% peak charging and 97.82% peak discharging; series/parallel battery-side reconfiguration | Research frontier for wide-range, reconfigurable modular conversion | 25 kW EV charging application, voltage range, cooling and system boundary differ sharply from PEBB-LV-01 |
| **IEEE SST DC/DC PEBB study** | Experimental comparisons of three modular SST DC/DC topologies; 98.55% reported peak efficiency | Research anchor for isolated modular building blocks and SST applications | SST building-block configuration and operating conditions are not a matched 48 V / 500 W product |

Sources and detailed caveats are listed in [SOURCES.md](./SOURCES.md).

## 2. Literature result at the exact 500 W power class

Wang et al. report a 500 W bidirectional CLLC laboratory prototype with ZVS and peak forward
efficiency of 97.6% (Electric Power Systems Research, June 2026, DOI 10.1016/j.epsr.2026.112834).
The authors use fundamental harmonic approximation for resonant gain and derive soft-switching
conditions as pre-design constraints. This is a more relevant same-rated-power research anchor than
the 25 kW SST papers.

The paper's abstract does not provide enough information to claim an apples-to-apples comparison
against PEBB-LV-01: exact tested voltage ratio, reverse-direction efficiency, cooling, boundary and
measurement uncertainty still need to be extracted. Treat 97.6% as a **reported peak forward result**,
not a full-envelope bidirectional benchmark.

## 3. What this says about our targets

The initial **97% Rev-A target is not enough to claim superiority**. ELSİS advertises more than 97% at full rated load for a 500 W bidirectional low-voltage buck-boost module. That product's public specification does not establish isolation, and its efficiency is a vendor claim rather than an independently reproduced measurement, but it is close enough to invalidate any casual "97% beats industry" statement.

The MPS data-centre module also advertises 97.8% peak efficiency at 500 W in a compact non-isolated LLC product. That is not the same function, but it shows the power-density and peak-efficiency bar in a related 48 V market.

Therefore:
- Keep ≥97% as a **functional Rev-A design target**, not a SOTA win.
- Keep ≥98% as a **stretch target**, not a claim.
- Add an internal competitive objective: beat the best directly matched baseline by more than the combined expanded measurement uncertainty while preserving the required isolation, bidirectional operation, protection and serviceability.
- Do not turn the above 97% figures into an apples-to-apples ranking. Their topology, voltage ratio, efficiency boundary, cooling and test conditions differ.
- A result that matches efficiency while adding galvanic isolation, replaceability and independent safety controls may still be a meaningful product improvement, but it must disclose the cost, volume and thermal trade-offs.

## 4. Higher-power CLLC research context

Two October 2026 publications further raise the modelling bar:
- Tao et al. report a 4.8 kW bidirectional CLLC PV-storage prototype at 97.45% peak charging and
  96.66% peak discharging efficiency. Their design ties FHA gain and gain monotonicity to inductive
  input impedance, magnetizing energy, Coss/dead-time commutation, electrothermal cooling and
  high-resolution PFM timing.
- Huang et al. report a 2 kW CLLC design using T-type impedance-network state constraints, with
  forward and reverse ZVS measurements on their prototype.

These are research comparators, not direct 500 W product matches. Their major implication is
methodological: a topology should only be selected from the intersection of gain, internal tank
state, impedance, commutation, current/thermal stress and achievable digital-control timing.
A nominal FHA gain curve alone is an insufficient design acceptance test.

The full T-type paper reports 95.7% maximum forward efficiency, 95.2% forward efficiency at
rated load, and 95.4% reverse efficiency for its own 2 kW prototype. It demonstrates forward/reverse
ZVS and a state-constrained design method, not a 97–98% efficiency record. The operating range is
150–300 V input and 75–167 V output, with 115 kHz resonance and 75–150 kHz switching.

## 5. The right head-to-head programme

### A. Establish three reference configurations

1. **Commercial functional baseline:** procure or obtain access to an ELSİS BIDIR module. Confirm exact SKU/revision, topology, isolation status, supported directions, thermal boundary and full test conditions with the manufacturer.
2. **Non-isolated reference circuit:** implement or simulate a matched low-voltage bidirectional buck-boost reference with the same 36–60 V port envelope and 500 W target. This isolates the efficiency/size/cost penalty or benefit of adding galvanic isolation.
3. **Luminous isolated candidates:** compare DAB and CLLC under the same power, voltage, direction, ambient, cooling and measurement boundary.

Do not use an isolated converter's input/output isolation rating as a proxy for a certified system-level safety case.

### B. Separate product dimensions

- **Conversion:** power-stage and complete-system efficiency maps, standby draw, input/output regulation, ripple and dynamic response.
- **Isolation:** working voltage, dielectric test, impulse withstand, creepage/clearance, partial discharge where applicable, and insulation aging.
- **Protection:** fault interruption, let-through current/energy, reverse current, short-circuit response, safe discharge and restart behavior.
- **Packaging:** full module kW/L and kW/kg including enclosure and cooling, thermal resistance and hot spots.
- **Serviceability:** time to identify, remove and replace a failed module; tool access; repair-induced requalification scope.
- **Economics:** BOM, manufacturing cost, yield, warranty, spares, field service, delivered energy and outage exposure.
- **Lifecycle:** energy losses, repair/reuse, critical-material content and end-of-life recovery.

A competitor can be better in one dimension and worse in another. Do not collapse the result into one weighted score.

### C. Direct-comparison protocol

- Obtain the exact competitor unit and revision where possible.
- Use the same calibrated analyzer, probes, cable/connector arrangement and environmental conditions.
- Match voltage, power, direction, load fraction, thermal equilibrium, airflow and auxiliary loads.
- Test forward and reverse independently. If the competitor cannot perform one direction, mark that comparison not applicable.
- Retain original datasheets, photographs of labels/revisions, instrument calibration and raw capture files.
- Calculate uncertainty for each measurement and for the difference between products.
- Publish full maps and worst-case corners, not just vendor peak points.
- Have an independent qualified laboratory reproduce any superiority claim intended for procurement or certification.

If a vendor does not disclose enough data to match a condition, keep its published figure as context and state **not comparable**.

## 6. Product-strategy consequence

We should not position PEBB-LV-01 as a generic 500 W brick that wins solely on efficiency. The strongest potential position is:

**An isolated, bidirectional, repairable and evidence-bearing low-voltage power module with a documented lifecycle and independent protection boundary.**

The value proposition is the combination of isolation, two-way power flow, replaceable building blocks, predictable service and reproducible qualification. Efficiency must be competitive, not the only differentiation.

## 7. Open evidence gaps

- Confirm ELSİS BIDIR module isolation status, exact control envelope and whether >97% is a typical, peak or full-load guarantee across the full range.
- Obtain the exact MPS and Vicor datasheets and test conditions for any chosen head-to-head.
- Find at least one commercial isolated **and** bidirectional 48 V-class module with public test data; current desk research has not yet established a fully matched product.
- Confirm actual prices, lead times, lifecycle status and second sources only when preparing procurement.
- Review with a power-electronics engineer whether the first isolated 500 W prototype is the best product-market wedge, or whether a lower-power bring-up stage should precede it.
