# Multiresolution AI Workload Trace and Sampling Adequacy v0.1

Research snapshot: 10 November 2026.

Status: **sampling/bandwidth contract and offline Rust screen authored; eight tests authored but not executed**. This is not an AI workload dataset parser, dataset validation result, sensor calibration, DAQ validation, anti-alias filter measurement, transient qualification or facility/grid qualification.

## 1. Public measured workload corpus

Two public 2026 datasets provide complementary scales of measured compute power:

- **NLR, Dataset of Generative AI Workload Power Profiles**, DOI 10.7799/3025227. NLR's catalogue describes 5/10 Hz traces (0.2/0.1 seconds) for LLM and image-generation inference/training on its Kestrel HPC platform, with raw/instantaneous and aggregate traces, configuration metadata and generated whole-facility profiles from a bottom-up event-driven model. The dataset's use/copy terms require the full notice to accompany copies and credit DOE/NLR/ALLIANCE; it prohibits endorsement implications and is supplied as-is. These terms must be preserved with any permitted copy.
- **Elsayed et al., Scientific Data (2026)**, DOI 10.1038/s41597-026-07496-6, describes over 1.8 million samples across 32 H100/B200 8-GPU node sessions and 40 RTX 3060 single-machine sessions. Its measurements include power, utilization, memory and temperatures. The paper describes a 50 Hz monitoring path (20 ms) and an approximately 100 ms HWiNFO path. The associated files are deposited on figshare, DOI 10.6084/m9.figshare.31654879. The dataset's own public repository README explicitly states CC BY-NC-ND 4.0; commercial use and sharing adapted/modified datasets are not permitted under that licence without separate rights-holder permission. Do not treat the data as available for Luminous commercial R&D or redistribution unless permission is obtained. The article licence and dataset repository licence should both be retained in the registry.
- **Vercellino et al., arXiv:2604.07345** describes 0.1-second H100 training/fine-tuning/inference power traces using MLCommons and vLLM benchmarks, then scales those traces into whole-facility profiles using a bottom-up event-driven model.

These sources are suitable for workload and facility research within their sampling and sensor limits. They do not measure processor-rail or package transients at microsecond or MHz scales.

## 2. Sampling is part of the evidence claim

A dataset cannot support a claim outside its measurement bandwidth. Sampling rate alone is not enough: sensor response, analog bandwidth, anti-alias filtering, timestamp jitter, clock synchronization, probe loading and measurement uncertainty all matter.

For uniformly sampled data, Nyquist frequency is half the sample rate. Thus:
- 50 Hz gives a nominal 25 Hz Nyquist limit.
- 10 Hz gives a nominal 5 Hz Nyquist limit.
- A 0.4 microsecond rise has a characteristic bandwidth scale far above either of those limits. A simple 0.35/rise-time estimate is approximately 875 kHz; this is only a measurement-planning heuristic, not a complete edge-spectrum requirement.

The new v0.1 auditor adopts the largest observed sample interval as a conservative effective-rate bound for timestamped traces:

f_sample,min = 1 / maximum_observed_gap

f_Nyquist,conservative = f_sample,min / 2

It also reports samples per cycle at the requested claim bandwidth. The caller declares the minimum samples per cycle; a ten-samples-per-cycle gate is an initial engineering screen for waveform-shape claims, not a universal standard.

The analogue measurement-chain contract must also cover the claimed passband and specify an anti-alias filter passband, stopband-start frequency, and stopband attenuation achieved before the conservative Nyquist limit. The audit does not inspect or measure a physical filter; it only checks the declared values.

## 3. v0.1 implementation

The Rust module tools/measurement-audit/src/sampling_adequacy.rs adds audit_sampling_adequacy.

It checks:
- strictly increasing timestamps and at least two samples;
- nominal interval and maximum relative interval jitter;
- maximum observed interval and the conservative equivalent sample rate;
- conservative Nyquist frequency;
- observed samples per cycle at the claimed bandwidth;
- declared instrument bandwidth;
- anti-alias filter passband, stopband, attenuation and placement below conservative Nyquist.

A failed requirement is retained as an explicit report limitation; malformed timestamps or invalid metadata are rejected. An adequate result means only that the declared sampling/bandwidth contract is internally consistent. It does not validate the physical sensor, calibration, analog filter transfer function, probe loading, or signal content.

Eight unit tests are authored, including a negative control that rejects a 50 Hz workload trace for a sub-microsecond processor transient claim. **Tests have not been executed.** No dataset archive was downloaded or parsed and no dataset adapter is claimed by this increment.

## 4. Multiresolution architecture

Do not downsample a high-rate trace and then present it as evidence that high-frequency events were absent. Preserve the original trace, raw digest and a distinct identity for every resampled derivative. Each derivative must declare the filter, aggregation window, phase, timestamp semantics and information intentionally discarded.

| Data layer | Typical purpose | Example evidence |
|---|---|---|
| Workload / accelerator telemetry, 5–50 Hz | Workload composition, energy, slow ramp, facility demand | NLR and Scientific Data profiles |
| Facility / PCC instrumentation, application-dependent | PCC variability, ramp, frequency and power-quality studies | Dedicated meter/DAQ and synchronized utility/grid model |
| Rack / bus electrical DAQ, kHz to MHz as required | Bus transients, storage response, converter interaction | High-bandwidth current/voltage measurement and calibrated sensors |
| VRM / package / die-side PI, MHz to GHz as required by the claim | Target impedance, transient response, resonance, coupling | VNA / injection / rail probes / EM model with exact reference plane |

The listed ranges are indicative engineering domains, not fixed requirements. Select the measurement method and bandwidth from the event rise time, spectral content, impedance magnitude, instrument transfer function and the requirement being tested.

## 5. Next research increments

1. Inspect the NLR archive README and exact file schema, then implement a source-specific adapter with license notice handling. The archive is approximately 1 GB and should not be added to the repository.
2. Inspect figshare metadata, sample files and data-specific license; implement a separate adapter only after exact column semantics and redistribution rights are established.
3. Add dataset registry entries with DOI, source/version, license, file sizes, raw SHA-256 and normalized derivative digests.
4. Create a held-out-session benchmark reporting energy, peak/P95 power, ramp, autocorrelation, band-limited PSD and workload-state composition separately.
5. Add explicit transformations between workload power and facility load, preserving measured data versus synthetic scaling/model output.
6. Connect facility traces to compute-preserving PCC replay (#382), but reject attempts to use low-rate workload data for chip/package transient claims.

## 6. Safety and evidence boundary

This increment is offline metadata/timestamp analysis only. It does not control workloads, converters, storage or grid equipment. Dataset quality reports do not validate Luminous tools. A sampling screen does not establish calibration, safety, grid-code conformance, product qualification or package integrity.

## Sources

- NLR Data Catalog: [Dataset of Generative AI Workload Power Profiles](https://data.nlr.gov/submissions/312), DOI [10.7799/3025227](https://doi.org/10.7799/3025227).
- Vercellino et al., [Measurement of Generative AI Workload Power Profiles for Whole-Facility Data Center Infrastructure Planning](https://arxiv.org/abs/2604.07345), arXiv:2604.07345 (2026).
- Elsayed, Al-Obaidi and Farag, [Characterization of high-resolution AI data center training workloads on single and multiple GPU nodes](https://www.nature.com/articles/s41597-026-07496-6), Scientific Data 13, 1268 (2026), DOI [10.1038/s41597-026-07496-6](https://doi.org/10.1038/s41597-026-07496-6).
- Figshare dataset: [10.6084/m9.figshare.31654879](https://doi.org/10.6084/m9.figshare.31654879).
