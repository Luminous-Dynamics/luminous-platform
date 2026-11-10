# Dataset Registry and Power-Flexibility Benchmark v0.1

Research snapshot: 10 November 2026.

Status: **machine-readable dataset registry and typed Rust policy/sampling validator authored; eleven Rust tests not executed**. No dataset binaries were downloaded, no adapters have been implemented, and no result is presented as an Luminous dataset measurement.

## 1. The data landscape

The public data landscape is stronger than it was even a month ago, but the sources have different roles and restrictions.

| Corpus | What it contributes | Resolution / status | Licence and caveat |
|---|---|---|---|
| NLR GenAI Power Profiles v2 | Measured Kestrel LLM/image-generation workload profiles plus a separate modeled facility product | 5/10 Hz; public archive, 1,021.3 MB | OEDI cross-list links CC BY 4.0; preserve attribution and no-endorsement |
| Elsayed et al. training workload dataset | Per-GPU/node power, utilization, memory, temperature and session data | 20 ms node path; 100 ms single-machine path; >1.8M samples | Dataset README states CC BY-NC-ND 4.0; not for commercial use or modified-data distribution absent permission |
| ELF Energy-Latency Frontier | Power-cap and DVFS sweeps paired with throughput and four-phase latency | Public CC BY 4.0; actual sampling interval and high-rate actuation trace format require inspection | Useful for power-flexibility/QoS frontier; the word high-rate is not a quantified bandwidth claim |
| ORNL Frontier Job-Centric Telemetry | Scheduler-linked jobs with node power, thermal, performance and some network series | 152,400 jobs; ~470.41 GB / 1,182,147 Parquet files | Globus access; public catalogue did not establish reuse/redistribution licence or sample bandwidth |
| LLM-PowerTrace | Controlled fine-tuning and serving configuration sweeps with per-GPU power and request traces | Preprint reports 1,788 training and 405 serving runs, 100 ms telemetry | Complete corpus described as released upon acceptance; verify data availability and data licence independently |
| Argonne JLSE Data Center Flexibility | Planned power-cap/clock/concurrency sweeps paired with request QoS | Research project, not a verified public downloadable dataset | Watchlist only, not an available benchmark corpus |

The NLR product must be split into at least two registry entries. Its Kestrel workload measurements are physical measurements; its bottom-up whole-facility examples are model-generated profiles. A common archive does not imply a common evidence class.

The LLM-PowerTrace preprint deserves attention for its metric choices: energy per token, P99 ramp rate, normalized spectral entropy and a per-device synchronization factor. These are useful external comparators. Its 100 ms telemetry has a 10 Hz sample rate and 5 Hz Nyquist. It is useful for slow workload dynamics, not a microsecond processor-rail transient.

## 2. Dataset reuse is a machine-checkable engineering constraint

The registry records licence status separately from download availability:
- CC BY 4.0 entries record commercial/adaptation/redistribution as permitted subject to attribution, change indication and no implied endorsement.
- CC BY-NC-ND 4.0 records commercial use as denied, derivative creation as denied and redistribution as conditional on the non-commercial licence terms.
- Unverified terms remain null. A public download link, DOI or Globus endpoint does not imply permission.
- Planned and pending-release data are unavailable until the release gate is verified.
- Access-controlled data require a separate access grant and still need licence verification.

The metadata validator is not a legal opinion. Before use, review the actual licence legal code, source notices, data-specific licence (not just the paper licence), third-party material and any institutional terms. The registry stores the source and date of the licence assertion so a later reviewer can re-check it.

## 3. Multiresolution claim eligibility

Sampling interval and bandwidth are part of the claim, not decorative metadata.

| Source path | Sample interval | Sample rate | Nyquist limit | Suitable first uses |
|---|---:|---:|---:|---|
| NLR faster path | 100 ms | 10 Hz | 5 Hz | Slow workload / facility demand |
| NLR slower path | 200 ms | 5 Hz | 2.5 Hz | Energy and slow facility modelling |
| Elsayed multi-GPU node | 20 ms | 50 Hz | 25 Hz | Workload and node power variability |
| Elsayed single machine | 100 ms | 10 Hz | 5 Hz | Energy and slow workload dynamics |
| ELF | Not established in reviewed catalogue metadata | Unknown | Unknown | Energy / power-flexibility study only, pending file inspection |
| ORNL | Not stated in reviewed catalogue | Unknown | Unknown | Job-centric energy / workload study, pending access and format review |
| LLM-PowerTrace | 100 ms nominal | 10 Hz | 5 Hz | Workload dynamics / grid-facing ramp features |

Nyquist is only a necessary sampling bound. It does not establish sensor bandwidth, anti-alias filtering, sample aperture, calibration, timestamp accuracy or transient amplitude fidelity. A 0.4 microsecond current edge needs a separate high-bandwidth, high-rate measurement chain and cannot be reconstructed from any of the workload datasets above.

The new Rust registry refuses a fast-transient use declaration unless the entry is a physical measurement and has:
- a known maximum sample interval;
- a known positive sensor bandwidth;
- verified anti-alias filter evidence;
- a declared fast-claim bandwidth within both conservative Nyquist and sensor bandwidth.

Unknown metadata is never substituted with a plausible default.

## 4. v0.1 implementation

The registry JSON is at schemas/dataset-registry-2026-11-10.example.json; the strict schema is schemas/dataset-registry-v0.1.schema.json.

The new Rust module tools/measurement-audit/src/dataset_registry.rs provides:
- validate_dataset_registry for required metadata, duplicate IDs, sampling/claim consistency and licence field consistency.
- DatasetRegistryEntry::conservative_nyquist_hz derived from the largest declared sample interval.
- DatasetRegistryEntry::supports_fast_transient_claim with physical-evidence, sensor-bandwidth and anti-alias prerequisites.
- DatasetRegistryEntry::decide_use for fail-closed commercial, derivative and redistribution decisions.

Eleven tests are authored for CC BY/CC BY-NC-ND decisions, unknown rights, controlled access, pending/planned availability, 20/100/200 ms Nyquist values, high-rate overclaim rejection, physical fast-claim requirements, separation of measured/modelled NLR products and invalid registry metadata. **They have not been executed.**

## 5. A better benchmark: power flexibility versus service quality

The next competitive unit is not simply joules per token or a maximum power reduction. It is the achievable power-flexibility envelope under a fixed useful-work/service contract.

For every control action (power cap, clock, batch/concurrency or workload admission), record:

1. **Electrical response:** absolute and percentage power shed, response latency, ramp, duration, recovery peak and rebound energy.
2. **Useful computation:** accepted output tokens, requests completed, training work to a fixed quality target, goodput and rejected/timed-out work.
3. **Service quality:** TTFT, p50/p95/p99 latency, tail SLO violation rate, output quality and workload mix.
4. **Energy:** total energy, joules per accepted output token / validated unit of work, cooling and storage-state correction where inside boundary.
5. **Grid-facing effect:** P99 ramp at declared delta, power spectrum/entropy, synchronized-device ramp factor and PCC response when actual facility measurements exist.
6. **Thermal/availability:** steady-state temperature, throttling, repeated-action degradation, failure/fallback and recovery.

The result is a Pareto frontier, not a single winner. A candidate that reduces power by 20% but misses latency or goodput is not a successful flexibility option.

### Proposed internal gates (not standards or demonstrated Luminous results)

- First controlled-flexibility milestone: at least 5% mean power shed for 60 seconds while retaining at least 99% useful goodput and limiting P99 latency increase to 5%.
- Stretch: 10% mean power shed at the same QoS contract.
- P99 ramp reduction: 20% versus an exactly matched uncontrolled workload trace.
- Energy per useful work unit: no worse beyond the combined measurement uncertainty unless the operator explicitly accepts a service/energy trade-off.
- Rebound: measure restoration peak and rebound energy; initial research target is no more than 10% of shed energy over the declared recovery window.
- Repeat the event under sustained thermal equilibrium; a short power cap is not proof of a sustainable operating point.

These gates are proposed experimental targets. They must be tested on the exact hardware and workload, not inferred from public trace data.

## 6. Where Luminous can outperform current practice

- Compared with energy-only benchmarks, retain the full power waveform, per-GPU synchronization, power cap action and QoS.
- Compared with workload-only schedulers, report power capability and response at a declared electrical boundary.
- Compared with converter-only optimization, account for workload, cooling, energy storage and useful work.
- Compared with unstructured simulation studies, tie all results to source datasets, exact configuration, licence, sample adequacy and requalification metadata.
- Compared with opaque vendor dashboards, provide a reproducible evidence packet and independent matched comparison.

LLM-PowerTrace already defines P99 ramps, spectral entropy and synchronization metrics. Luminous should implement matched calculations and add explicit PCC, storage-state, service-quality, and recovery/rebound accounting rather than claim those existing metrics as novel.

## 7. Next increments

1. Inspect the NLR v2 archive README/schema and build a source-specific adapter without committing the archive.
2. Inspect ELF's actual file structure and high-rate actuation timebase; its dataset is the best near-term power-flexibility source.
3. Establish ORNL Globus access/terms before analysis; the dataset's 470 GB scale calls for selective query/partition processing rather than cloning it into Git.
4. Recheck LLM-PowerTrace release and dataset licence after the stated acceptance/release gate.
5. Build the offline power-flexibility Pareto evaluator and a trace-based replay protocol, then validate it using permitted datasets.
6. Add multi-site synchronization/correlation and PCC/EMT models only after the workload-to-facility link is evidenced.

## 8. Safety and claim ceiling

- Dataset registry validation is metadata validation, not inspection of data binaries or legal advice.
- Unknown licence rights block reuse/redistribution decisions.
- Modelled facility profiles are not physical PCC measurements.
- Low-rate workload measurements do not qualify VRM/PDN transients.
- A numerical power-flexibility frontier is not proof of grid service eligibility or utility acceptance.
- No workload control, grid dispatch, 800 V energization, storage fault injection or high-energy test is authorized.

## Sources

- NLR Data Catalog: [GenAI Workload Power Profiles](https://data.nlr.gov/submissions/312), DOI [10.7799/3025227](https://doi.org/10.7799/3025227); cross-listed [OEDI CC BY 4.0 record](https://data.openei.org/submissions/8651).
- Elsayed et al., [Scientific Data article](https://www.nature.com/articles/s41597-026-07496-6), DOI 10.1038/s41597-026-07496-6; [dataset README and licence](https://github.com/Ahmed-Elsayed95/High-resolution-AI-Data-Center-Training-Workloads-Dataset/blob/main/README.md).
- [ELF Energy-Latency Frontier](https://doi.org/10.5281/zenodo.23074732), Zenodo; rights summary in the [NLM Dataset Catalog](https://datasetcatalog.nlm.nih.gov/dataset?q=0003346687).
- ORNL, [Frontier Job-Centric Telemetry](https://doi.org/10.13139/OLCF/3013979).
- Wang et al., [LLM-PowerTrace](https://arxiv.org/abs/2610.38315), arXiv:2610.38315.
- Argonne JLSE, [Data Center Flexibility Dataset project](https://www.jlse.anl.gov/projects/benchmarking/data-center-flexibility-dataset).
- Creative Commons, [CC BY 4.0 deed](https://creativecommons.org/licenses/by/4.0/).
- Creative Commons, [CC BY-NC-ND 4.0 deed](https://creativecommons.org/licenses/by-nc-nd/4.0/).
