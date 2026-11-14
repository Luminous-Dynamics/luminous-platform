# Grid-forming capability passport and multi-fidelity test coverage — v0.1

**Tracking:** [Issue #474](https://github.com/Luminous-Dynamics/luminous-platform/issues/474)  
**Transition state graph:** [PR #455](https://github.com/Luminous-Dynamics/luminous-platform/pull/455)

**Status:** typed claim-to-test coverage/evidence-tier validator and synthetic fixtures. No inverter waveform, hardware, site, conformity or GFM performance test has been executed.

## Research basis (2026-11-14)

- Tan et al., 19 October 2026, review 16 representative industry-oriented GFM tests and supplementary weak-grid, unbalanced-fault and multi-vendor testing. It emphasizes initial operating point, disturbance, boundary, current/energy limits, response criteria and recovery as part of every test definition: https://www.mdpi.com/1996-1073/19/20/4966
- IEEE Std 2800-2022 is the active transmission-level IBR interconnection standard: https://standards.ieee.org/ieee/7003/10453/
- IEEE 2800.2-2026 (published 2026-06-30) supplies IBR plant test and conformity-assessment procedure guidance: https://standards.ieee.org/ieee/2800.2/10616/
- IEEE P2800.1 is an active PAR approved 2025-12-10 for GFM-specific capabilities, performance and model-based test procedures; it is not yet a published final practice: https://standards.ieee.org/ieee/2800.1/12387/
- NLR/UNIFI's 1-MW multi-vendor test platform includes seven GFM inverters from six vendors, multiple GFL units and synchronous generation: https://docs.nlr.gov/docs/fy26osti/96886.pdf

IEEE 2800 interconnection evidence does not establish every GFM-specific capability. Phase-angle-jump response, inertia behaviour, autonomous island formation, GFM/GFL switching and black start need their own function-specific criteria and evidence.

## Claim-specific capability mapping

A claim request lists the functions actually being asserted. Each capability expands to its mandatory tests and metrics. The validator does not infer a broad GFM badge from generic IBR interconnection evidence. Products need not claim every function; a test outside the declared scope can be marked not applicable with an approved rationale, but it is never counted as a pass.

## Test matrix

**16 core equipment/plant tests:** voltage/frequency adaptability; power quality; active-power control; reactive-power control; primary frequency regulation; voltage/reactive-power regulation; inertia response; power-oscillation damping; voltage tolerance (LVRT/HVRT); phase-angle-jump tolerance; GFM/GFL control-mode switching; equipment black start; black start from zero voltage; islanded load pickup; grid loss and resynchronization; load-following and frequency/voltage droop.

**3 supplementary tests:** weak-grid SCR/X/R variation; unbalanced fault/negative-sequence response; multi-converter/multi-vendor interaction.

**Separate system-scale profile:** system disturbance response (generation/load loss, interconnector outage, HVDC blocking or system separation). It is distinct from the 19 equipment/plant test IDs.

## Evidence ladder

- **SyntheticCoverageOnly:** validates the profile and coverage logic; never physical evidence.
- **ModelValidated:** modelled RMS/EMT/hybrid simulation. Does not establish hardware performance.
- **ControllerHIL:** measured controller-HIL evidence; does not establish power-hardware performance.
- **PowerHardwareTested:** measured PHIL or PCS laboratory/grid-emulator evidence.
- **SiteDemonstrated:** current measured site-commissioning or field evidence.
- **IndependentSiteClaim:** witnessed/accredited site/field evidence.

Every evidence record binds product configuration, scenario manifest, acceptance criteria, evidence bundle, measurement boundary, raw trace and mandatory metric IDs. Physical methods additionally require calibration and traceable timebase references. Witnessed/accredited evidence requires a witness reference.

The validator checks digest shape and declared metric identifiers; it does not authenticate raw bundle bytes or decide whether numerical acceptance thresholds are technically adequate. Mycelix remains responsible for signed receipt verification and revocation.

## Synthetic controls

- A baseline IBR evidence set is deliberately insufficient for inertia, phase-angle-jump and autonomous black-start claims.
- A complete 19-test synthetic set demonstrates coverage but is eligible only for SyntheticCoverageOnly.
- A model-only record cannot satisfy a power-hardware claim.
- Missing mandatory metrics, stale evidence, uncalibrated physical records, unsupported methods or current failed repeats invalidate the affected test.
- The system-disturbance profile remains separate from the 19-test equipment/plant matrix.

## Benchmark targets

- 100% mapping from each claimed capability to required test IDs.
- Zero automatic GFM credit from generic IBR interconnection evidence.
- 100% mandatory metric IDs present in every claimed passing test.
- Zero current failed runs hidden by passing repeats.
- 100% not-applicable records have rationale and approval; none overlap claimed capabilities.
- 100% physical evidence has current calibration and traceable timebase references.
- Multi-vendor: target at least three vendors with GFM/GFL mixing in a 1-MW-class setup where facilities permit. NLR/UNIFI's seven-GFM/six-vendor platform is a comparator, not a universal product requirement.
- Report minimum demonstrated SCR/X/R envelope, peak/sustained current, current-limit entry/exit, synchronism, recovery and DC energy margin.
- Compare model/EMT traces with CHIL/PHIL/PCS lab traces using preregistered error tolerances, bandwidth and operating envelopes.

## Claim ceiling

`GFM_CAPABILITY_EVIDENCE_COVERAGE_ONLY_NOT_STANDARD_CONFORMITY_OR_FIELD_SUPERIORITY`

No IEEE 2800 or P2800.1 conformity, GFM product performance, black-start, ride-through, multi-vendor interoperability or SOTA superiority is asserted by this software.
