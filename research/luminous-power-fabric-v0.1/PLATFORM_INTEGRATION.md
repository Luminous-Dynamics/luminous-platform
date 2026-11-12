# Luminous Power Fabric platform integration contract v0.1

**Status:** integration proposal; it does not assert that these integrations are implemented.

## 1. Authority separation

The programme uses five distinct roles:

- **Research / synthesis:** proposes topologies, components, hypotheses, and experiment priorities.
- **Geometry / product lifecycle:** owns exact physical configurations, placement, BOM and revision.
- **Simulation / replay:** evaluates declared models and test scenarios.
- **Local deterministic control:** runs the bounded control loop of the physical power module.
- **Independent protection / evidence verification:** constrains unsafe states and assesses whether
  claims are supported by the exact evidence package.

No model or distributed service may collapse these roles into one trust domain.

## 2. System responsibilities

### Symthaea

Permitted:
- Synthesize manufacturer and peer-reviewed data with source provenance.
- Generate candidate circuit/topology search spaces and component alternatives.
- Rank Pareto trade-offs under declared objectives and hard constraints.
- Propose experiment plans and explain observed discrepancies.
- Identify missing/contradictory assumptions and request a human engineering decision.

Not permitted:
- Directly gate power semiconductors or command a live converter.
- Change current/voltage/temperature protection limits at runtime.
- Convert predicted reliability or simulated performance into measured claims.
- Treat incomplete datasheets, stale telemetry or unknown component identity as safe.

### Symtropy

Permitted:
- Model switching, control, electrical network, thermal, magnetic and mechanical phenomena.
- Replay fault scenarios and parameter variations.
- Compare model predictions with bench data and compute residuals.
- Design controller-HIL / PHIL scenarios and record model revision.

Required:
- State assumptions and model fidelity.
- Identify idealized components and omitted parasitics.
- Preserve solver, timestep, convergence, parameter and seed metadata.
- Keep simulation result, model validation and physical measurement status separate.

Not permitted:
- Declare a physical device safe because the model is stable.
- Claim an insulation, thermal, EMI or reliability test has passed without physical evidence.

### Sol-Atlas

Permitted:
- Own versioned geometry, board/enclosure/thermal layouts and wiring routes.
- Track mechanical stack-up, clearances, mass, volume and service access.
- Bind every generated drawing and manufacturing artifact to exact configuration/BOM revision.
- Compare repair procedures and replacement-module envelopes.

Required:
- Keep dimension source and tolerance explicit.
- Represent cable/busbar and insulation keep-outs.
- Tie all geometry to exact PCB, component and enclosure revisions.
- Mark design changes that invalidate thermal, EMI, dielectric or mechanical evidence.

### Mycelix

Permitted:
- Link manufacturer, exact part, lot/date code, approved alternate and procurement evidence.
- Bind firmware, calibration certificates, test files and service records to hardware identity.
- Verify evidence digest, sequence, issuer identity and declared source/control domain.
- Preserve negative findings, superseded evidence and unknown statuses.

Required:
- Distinguish bytes-authenticated from source-authenticated and physically validated.
- Do not treat different keys/processes/device IDs as proof of physical independence.
- Require reconciliation for unknown test or service outcomes before any new mutation.
- Preserve component and instrument provenance through repair and replacement.

### Embedded deterministic controller and protection

- The local fast control loop remains deterministic, bounded, versioned and tested.
- Hardware protection is independent of the AI, the management network and, where practical,
  the primary MCU's software decision.
- A remote command is only a bounded request; local hardware limits decide whether it is admissible.
- Communications loss cannot turn off required local protection.
- Firmware update or rollback cannot occur while power transfer is in an unsafe indeterminate state.

## 3. Evidence record minimum

Each physical measurement run should carry:

- run_id, device_id, hardware revision, PCB revision and BOM digest.
- Power-stage serial / lot and all safety-critical semiconductor, capacitor and magnetic part refs.
- Firmware and controller configuration digest.
- Instrument make/model/serial, calibration certificate and expiry/status.
- Sensor/probe type, range, bandwidth, location and uncertainty.
- Test operator, reviewer, timestamp source, timebase and lab environmental conditions.
- Port voltage/current, transferred power, direction, load profile and operating mode.
- Raw data URI, byte digest, parser/auditor revision and result digest.
- Test procedure revision, acceptance thresholds and deviations.
- Result status, failed checks, invalid samples, exclusions and reviewer disposition.
- Model/geometry revision used to interpret the result.

A signed test file proves the recorded bytes were signed by a key; it does not alone prove the sensor
was correct, the calibration was valid, the test was performed as stated, or the signer was independent.

## 4. Proposed measurement CSV format

A raw capture is one uninterrupted, steady-state window with fixed instrument configuration. Use SI
units and decimal points. No thousands separators. Empty, NaN, infinity or missing numeric fields are
invalid. Store the original file unchanged and bind its SHA-256 in the run manifest.

Header:

time_s,port_a_voltage_v,port_a_current_a,port_b_voltage_v,port_b_current_a,case_temp_c

Convention:
- Both port currents are positive **into** the converter.
- For A→B operation, port A power should be positive and port B power negative.
- For B→A operation, port B power should be positive and port A power negative.
- Record intended direction outside the CSV in the immutable run manifest.
- Use the same timebase for all channels; if channels are sampled by separate instruments, record
  synchronization method and uncertainty.
- Capture only the predeclared steady-state window. Store transient captures as a separate dataset
  with a distinct procedure.

The auditor must reject a run if timestamps are not strictly increasing, values are non-finite,
voltage is outside the declared instrument range, power signs conflict with the stated direction,
or integrated input/output energy is non-positive. It may report outliers but must not silently delete
them or infer missing samples.

## 5. Promotion boundary

A model-generated design may enter a controlled design review, not a live control plane. A prototype
may be energized only after design and lab reviews. Firmware, BOM, geometry, calibration or protection
changes suspend affected claims until the corresponding verification is repeated.

The platform is an evidence and lifecycle layer around physical engineering. It is not an electrical
protection device, certification body, accredited test laboratory or utility approval authority.
