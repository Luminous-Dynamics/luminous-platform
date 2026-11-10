# Frequency-Dependent PDN Target and Impedance Audit v0.1

Research snapshot: 10 November 2026.

Status: **offline analytical screening module; eight Rust unit tests authored but not executed**. This is not an electromagnetic field solver, S-parameter importer, de-embedding engine, control-loop stability analysis, physical measurement, package design approval or power-integrity qualification.

## 1. Why the scalar target is insufficient

The scalar estimate

Z_target = ΔV_allowed / ΔI

is a useful first-order budget. It does not describe whether a PDN stays below its target impedance over the frequencies excited by the processor's current spectrum. A time-domain load-step report also cannot establish the complete frequency-domain impedance envelope unless the excitation, measurement bandwidth, system linearity and reference planes are sufficiently characterized.

The current processor-side challenge is becoming more severe:

- Google's 2026 paper *Towards 10kAmp AI Chip* discusses accelerator currents exceeding 10,000 A and couples PI, SI, thermal, mechanical/warpage and manufacturing limitations when comparing lateral and vertical power delivery.
- Signal Integrity Journal's 2026 PDN-design review describes the shift toward tens-of-micro-ohms impedance and the challenge of spatial variation when lateral plane impedance becomes comparable to the PDN target.
- Keysight's two-port VNA application note describes shunt-through methods for measuring very low impedance and why a conventional one-port setup can be dominated by mismatch and fixture effects.
- Tektronix documents wide-band impedance characterization using VNAs and complementary low-frequency rail impedance methods.
- Cadence's recent interconnect analysis emphasizes probe coupling and finite lateral impedance as material sources of measurement and modelling error.

The implication is not that every current accelerator has the same target impedance. It is that every claim must state its current spectrum, rail voltage budget, bandwidth, geometry and measurement plane.

## 2. Target impedance from a declared spectral budget

The v0.1 input separates:
- Nominal rail voltage.
- Available AC ripple RMS budget, which must already be net of DC load-line, regulator ripple, measurement and other reserved terms.
- Processor current ripple RMS per declared frequency bin.
- A voltage-ripple RMS allocation for each frequency bin.

For frequency bin k:

Z_target(f_k) = V_ripple,budget(f_k) / I_ripple,rms(f_k)

The v0.1 implementation treats the bins as orthogonal, non-overlapping RMS spectral components and combines their allocations by root-sum-square:

V_allocated,rms = sqrt(sum_k(V_ripple,budget(f_k)^2))

The allocation must not exceed the declared available AC ripple budget. If the source spectrum uses overlapping bands, has material leakage, or does not define a coherent observation window, the RSS assumption must be revisited; the tool does not repair an invalid spectral decomposition.

This avoids using the entire rail tolerance independently at every frequency. It also avoids assuming that all of the rail's tolerance is available for AC PDN ripple.

## 3. Complex impedance comparison and uncertainty

For each exactly matched frequency, the tool accepts complex self-impedance:

Z(f) = R(f) + jX(f), |Z(f)| = sqrt(R(f)^2 + X(f)^2)

It accepts separate standard uncertainties for the real and imaginary components. The conservative screening bound is:

|Z|_upper = |Z| + k * (sigma_R + sigma_X)

where k is an explicit positive coverage factor supplied by the caller. This linear sum avoids assuming covariance between the real and imaginary uncertainty components. It is a conservative engineering screen, not a complete metrological uncertainty model or a coverage statement independent of the uncertainty distributions.

The point is within its declared envelope only when:

|Z|_upper / Z_target <= 1

The report gives the worst ratio and exact frequency. It does not hide a narrow-band exceedance inside a band average or a single global scalar.

## 4. Anti-resonance candidate screening

The implementation flags an interior point as a local impedance-peak candidate when:
- its measured magnitude is strictly above the immediately adjacent points; and
- its ratio to the larger adjacent magnitude meets the caller's explicit threshold.

This is only a sampled local-peak flag. It does not identify the physical cause, characterize Q, establish resonance bandwidth, or prove anti-resonance between sparse samples. A denser scan, phase, component topology, de-embedding and model correlation are needed for attribution. Smoothing must not erase a narrow peak without preserving the raw scan.

## 5. Reference plane and evidence class

Every scan carries:
- configuration identity;
- reference-plane identity, e.g. VRM output, board sense point, package substrate or die-side sense port;
- instrument lower and upper bandwidth;
- evidence kind: synthetic, analytical, simulated, vendor-reported or physical measurement;
- calibration reference where physical measurement is claimed;
- explicit indication whether the scan has been de-embedded to the named reference plane.

The module does not perform de-embedding. It computes the numerical comparison for supplied values and separately reports whether the reference plane is confirmed and whether the physical-measurement context is complete. A calibration identifier is not proof that calibration is current, valid or traceable. Content hashes, signed manifests, instrument identity and reviewer authority are part of the separate evidence-manifest workstream (#384).

## 6. Industry positioning and benchmark protocol

The relevant measurement methods include two-port shunt-through VNA methods, wide-band VNA scans and low-frequency injection/rail-impedance methods. They are complementary; instrument selection depends on the DUT impedance, target frequency range, fixture, dynamic range and measurement objective.

Luminous should benchmark methods before comparing architectures:

| Comparison | Required matching |
|---|---|
| Lateral vs vertical delivery | Same die-side rail, current spectrum, voltage budget, useful compute, cooling and package boundary |
| Decoupling variants | Same reference plane, PCB/package stack, capacitor model and mounting parasitics |
| Impedance scan vs simulation | Same geometry, ports, plane, frequency range and boundary conditions |
| Frequency scan vs load-step response | Declared bandwidth and a validated model linking domains |
| Package candidate vs rack/facility outcome | Same workload trace and energy boundary, with package heat and cooling included |

Proposed internal research gates (not industry standards or measured Luminous performance):
- uncertainty-aware upper-bound-to-target ratio no greater than 1.0 at every declared frequency point;
- no unreported local peak above the declared prominence threshold;
- publish raw scan, frequency grid, reference plane, bandwidth, uncertainty and de-embedding status;
- correlate frequency-domain predictions with independent time-domain transient evidence;
- compare total decoupling volume, loss, temperature, warpage, yield and serviceability at equivalent delivered useful work.

## 7. v0.1 implementation

The new Rust module at tools/measurement-audit/src/pdn_frequency.rs provides:

- audit_pdn_frequency_response: validates an explicit current-spectrum and complex-impedance frequency grid; derives per-frequency target impedance; verifies RSS ripple allocation; computes uncertainty-aware magnitude bounds; flags local peaks.
- PdnFrequencyAuditReport::numerical_screen_pass: reports only whether the declared numerical target envelope and budget are internally satisfied.
- PdnFrequencyAuditReport::physical_measurement_context_complete: separately indicates whether evidence is explicitly physical, a calibration reference is present and de-embedding to the named plane is asserted.

The implementation requires exact, strictly increasing frequency-grid equality between the target spectrum and impedance scan. It deliberately does not interpolate, extrapolate, smooth, import Touchstone/S-parameters, calculate de-embedding, solve field equations, evaluate VRM loop stability or reconstruct time-domain responses.

Eight unit tests are authored for target derivation, ripple-budget overflow, frequency exceedance, local-peak detection, uncertainty bounds, frequency-grid mismatch, ordering, physical evidence metadata and non-finite/out-of-band inputs. **They have not been executed.**

## 8. Next research increments

1. Add a standards-aware Touchstone 1.x/2.x importer with strict option-line, reference impedance, port ordering and frequency-unit handling; preserve exact raw artifact bytes and digest.
2. Add calibrated de-embedding adapters for 2x-Thru, open/short and fixture characterization. Keep reference-plane transformations explicit and reproducible; never imply that parsing is de-embedding.
3. Add spatial multiport impedance Z_ij(f), transfer impedance and measurement-location comparison. The single-port self-impedance screen cannot represent spatial PDN coupling.
4. Add band-limited current-spectrum ingestion from measured GPU/processor traces, including window, leakage, averaging, phase and uncertainty metadata.
5. Add a correlated frequency/time-domain workflow with independent cross-validation. Do not assume linear time-invariant behaviour when regulator control, nonlinearity or operating-point changes invalidate it.
6. Add electrothermal/mechanical co-analysis and evidence-linked package geometry once validated solver outputs and physical boundary conditions exist.

## 9. Safety and claim ceiling

- This is offline read-only analysis and does not command a regulator or any hardware.
- A numerical envelope pass does not prove loop stability, transient compliance, EMC/SI compatibility, manufacturability, thermal reliability or product safety.
- An anti-resonance candidate is not a physical diagnosis.
- An un-de-embedded scan is not a die-side or package-side impedance claim.
- No package fabrication, 800 V energization, high-energy testing, destructive testing or service action is authorized.

## Sources

- Google Research / IEEE, 23 July 2026: [Towards 10kAmp AI Chip: A System Perspective from Lateral to Vertical Power Delivery](https://research.google/pubs/towards-10kamp-ai-chip-a-system-perspective-from-lateral-to-vertical-power-delivery/).
- Signal Integrity Journal, 2026: [Evolution of the PDN Design Process: Explaining the Second Paradigm Shift](https://www.signalintegrityjournal.com/articles/4286-evolution-of-the-pdn-design-process-explaining-the-second-paradigm-shift).
- Keysight: [Ultra-Low PDN Impedance Measurements Using 2-Port VNAs](https://www.keysight.com/ca/en/assets/7018-08474/application-notes/5989-5935.pdf).
- Tektronix: [Analyzing Power Integrity on a Power Distribution Network](https://www.tek.com/en/solutions/application/analyzing-power-integrity-on-pdns).
- Cadence: [Impact of Finite Interconnect Impedance Including Spatial and Domain Comparison of PDN Characterization](https://www.cadence.com/en_US/home/resources/white-papers/impact-of-finite-interconnect-impedance-including-spatial-and-domain-comparison-of-pdn-characterization-wp.html).
- IEEE Standards Association: [P4200 — Data Center Interconnection](https://standards.ieee.org/ieee/4200/12632/).
