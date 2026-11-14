# Grid-forming capability passport and test comparability — v0.1

**Tracking:** [Issue #484](https://github.com/Luminous-Dynamics/luminous-platform/issues/484) · [Transition qualification PR #455](https://github.com/Luminous-Dynamics/luminous-platform/pull/455)

**Status:** synthetic claim-to-test-to-evidence contract and typed offline validator. No GFM hardware, converter, controller, grid emulator or site was tested by this work.

## Research baseline (14 November 2026)

- Tan et al., *Review of Testing and Validation Methodologies for Grid-Forming Performance of Inverter-Based Resources*, 19 October 2026, synthesises 16 representative GFM tests and supplementary weak-grid, unbalanced-fault, multi-vendor and system-level validation: https://www.mdpi.com/1996-1073/19/20/4966
- VDE FNN *Technical requirements for grid-forming capabilities including provision of inertia*, Version 2.1, April 2026, gives equipment-class-specific requirements for voltage-source behaviour, frequency/phase-angle response, current limitation, damping, harmonic passivity and impedance: https://www.vde.com/resource/blob/2473026/ad76f904abf220d2c2759cb9a33a283d/vde-fnn-guideline-grid-forming-capabilitiesv2-1-data.pdf
- IEC TS 62898-3-5:2026 provides HIL/commissioning/periodic testing context for microgrid control systems: https://webstore.iec.ch/en/publication/84175
- NLR's 2025 Borrego Springs work provides a physical PHIL comparator for planned islanding and islanded load steps: https://research-hub.nlr.gov/en/publications/power-hardware-in-the-loop-experiments-of-a-microgrid-with-a-grid-2/

These documents are not interchangeable standards. Declare jurisdiction, connection code, equipment class, edition, clauses and project acceptance contract.

## Capability taxonomy

The v0.1 taxonomy covers the 16 representative test families: voltage/frequency adaptability; power quality; active/reactive command control; primary frequency regulation; voltage/reactive-power regulation; inertia/RoCoF response; power-oscillation damping; voltage tolerance; phase-angle-jump tolerance; GFM-GFL switching; black-start capability; black start from zero voltage; islanded load pickup; grid loss/resynchronisation; and load-following with coordinated frequency/voltage droop. Supplementary families cover weak-grid/SCR sweep, unbalanced fault/negative sequence, multi-vendor interoperability and system-level disturbance.

## Passport method

A claim binds exact product configuration, topology, operating envelope, test boundary, acceptance-contract identity/edition and required test/evidence-stage pairs. Each protocol binds initial operating point, disturbance, measurement boundary, calibration/timebase digests and metric criteria. Each observation binds the exact protocol digest, evidence stage/class, raw bundle, analysis build, verification-receipt reference and measured metrics with absolute uncertainty.

Vendor, modelled, synthetic, measured, witnessed and accredited evidence map to distinct stages. This is not a linear ranking: a site result does not automatically replace a controlled lab test when the disturbance cannot safely be reproduced onsite. Unit-level evidence cannot silently become a plant/system claim.

## Uncertainty-aware acceptance

Pass requires the full measurement interval inside the permitted range. Fail means the full interval is outside. Inconclusive means uncertainty overlaps an acceptance boundary. Missing metrics/context are invalid; observations bound to an old protocol digest are stale. Claims qualify only when every required test/stage pair meets its declared pass count and failure allowance. Failed, inconclusive, invalid and stale results remain visible.

## Benchmark targets

- 100% claimed capabilities map to frozen test protocols and acceptance contracts.
- 100% metrics carry unit, boundary, sample window and uncertainty.
- Exact configuration/topology/operating-envelope/initial-condition/disturbance identity.
- Zero physical qualification from vendor, synthetic or unvalidated model evidence.
- 100% analysis replay from frozen raw-data root and analysis build.
- Cross-lab comparison uses the same reference model, disturbance, measurement definition and acceptance contract; report inter-lab deltas and uncertainty.
- Multi-vendor testing measures P/Q sharing, circulating power, oscillatory modes/damping, stability margins and SCR sensitivity.
- Unit-to-plant-to-system transfer requires a validated model domain and corresponding system tests.

FNN v2.1 numeric criteria are jurisdiction/equipment/version-specific. Store the governing contract digest rather than treating one jurisdiction's thresholds as global SOTA.

## Claim ceiling

`SYNTHETIC_GFM_CAPABILITY_PASSPORT_ONLY_NOT_PHYSICAL_QUALIFICATION_OR_GRID_CODE_CONFORMITY`

The module does not authenticate raw bytes, verify signatures, validate instruments, execute simulations, control hardware, perform HIL/PHIL, test a site or determine conformity. Mycelix remains external evidence-verification owner; Symtropy owns model/runtime/CHIL/PHIL execution.
