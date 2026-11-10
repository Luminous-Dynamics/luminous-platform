# Fleet-Capacity-Aware Joule Point Benchmark v0.1

Research snapshot: 10 November 2026.

Status: **offline fixed-fleet and capacity-scaled evaluator authored; eleven Rust unit tests authored but not executed**. This evaluator does not run a scheduler, set GPU power caps, predict a complete production cluster, model network contention, or qualify a facility.

## 1. Why per-GPU power is not fleet power

The current power-flexibility Pareto evaluator (#392) compares per-run PCC power, useful work, latency, SLO, storage, and rebound. A next-order question is missing: when per-GPU goodput falls, how many additional GPUs must be installed to deliver the same offered workload?

The distinction is essential:
- Per-GPU power may decline while fleet GPU board power increases.
- Preserving aggregate throughput may require integer GPU-count expansion.
- Extra GPUs change rack density, host/network overhead, cooling and capital.
- A board-level saving is not a facility/PCC saving unless the additional terms are measured or modelled explicitly.

### External frontier: ELF and the Joule Point

The October 2026 **ELF: Energy-Latency Frontier** dataset (Apartsin and Aperstein, Zenodo DOI 10.5281/zenodo.23074732, CC BY 4.0) covers 20 inference models on four NVIDIA GPU platforms (T4/L4/A10G/A100), power-cap and graphics-clock/DVFS sweeps, achieved throughput, and a four-phase latency split. The catalogue describes:
- \`data/elf_master.csv\`: 2,636 settled operating-point rows and a column dictionary;
- 6,488 measurement windows, typically two or three consecutive two-second windows per operating point;
- a first post-cap window affected by NVML's trailing-average settling pause, which the authors exclude in settled analyses while preserving raw windows;
- a separate high-rate power-cap actuation trace whose actual sample interval and measurement bandwidth remain to be inspected.

The companion paper **The Joule Point: an Energy-Optimal Operating Point for AI Inference** (Apartsin and Aperstein, arXiv:2609.27926, 23 August 2026) reports, for its tested large GPUs, an energy-per-inference minimum around 43–46% of rated power, roughly 25–33% lower energy per inference with approximately 1.2x request latency, and about 1.2x as many GPUs needed to hold aggregate throughput. Its fleet simulation reports 18–45% lower energy per served job under its studied power-budget scenarios.

These figures are research results for the authors' tested configurations and model assumptions. They are not universal cap recommendations, independent industry-standard limits, or Luminous measurements.

## 2. Two different deployment questions

### Fixed-fleet test

Keep the existing GPU count unchanged:
- Does the candidate still provide enough aggregate goodput for the offered request rate plus headroom?
- Does measured completed work meet the work-loss gate?
- Do P99 latency, TTFT, and SLO remain within the same contract?
- What happens to measured GPU-board power and any known server/rack overhead?

This tells the operator whether a cap is usable with existing installed capacity.

### Capacity-scaled projection

Calculate the minimum integer GPU count required to meet demand plus reserve:

\[
N_{\mathrm{required}} =
\left\lceil
\frac{\lambda_{\mathrm{offered}}(1+h)}
     {g_{\mathrm{GPU}}}
\right\rceil
\]

where \`lambda_offered\` is offered useful-work rate, \`h\` is the declared headroom fraction, and \`g_GPU\` is measured per-GPU goodput at the candidate operating point.

The model projects linearly from measured single-GPU goodput to fleet capacity. It does not simulate queueing, cross-GPU interference, network saturation, collective communication, scheduler fragmentation, or thermal coupling. The result is therefore a capacity estimate, not measured scaled-fleet performance.

## 3. Power and energy boundaries

GPU board power remains separate:

\[
P_{\mathrm{GPU,fleet}} = N P_{\mathrm{GPU}}
\]

A declared total-system estimate may be computed only when both overhead terms and their evidence class are supplied:

\[
P_{\mathrm{system}} =
N(P_{\mathrm{GPU}}+P_{\mathrm{support/GPU}})
+P_{\mathrm{fixed}}
\]

This prevents the evaluator from silently assuming that CPU, memory, network, cooling, power conversion, or fixed facility overhead is zero. The fixed overhead is supplied separately for the fixed-fleet and capacity-scaled scenarios; the evaluator does not reuse one value or automatically scale it with GPU count.

For the total-system boundary:

\[
E_{\mathrm{effective}} =
P_{\mathrm{system}}T -
(E_{\mathrm{stored,final}}-E_{\mathrm{stored,initial}})
\]

\[
E_{\mathrm{per\,work}} =
\frac{E_{\mathrm{effective}}}{W_{\mathrm{accepted}}}
\]

The storage correction is applied to the declared total-system boundary, not to the GPU-board component-energy figure. GPU board energy per work is reported separately.

Capital expense is reported only if a positive per-GPU cost, explicit currency and cost-source reference are supplied. Unknown costs remain unknown.

## 4. Synthetic fleet arithmetic

Consider a deliberately simple example:
- offered demand: 80 work units/s;
- baseline GPU: 10 work units/s and 400 W per GPU;
- candidate GPU: 8 work units/s and 200 W per GPU;
- duration: 60 seconds;
- capacity headroom: 0%;
- support overhead: 50 W per GPU;
- fixed overhead: 500 W.

The baseline needs 8 GPUs. A 20% per-GPU goodput decline requires:

\[
\lceil 80/8 \rceil = 10 \text{ GPUs}
\]

That is 25% more GPUs. In the capacity-scaled model:
- baseline GPU board power: 3,200 W;
- candidate GPU board power: 2,000 W;
- baseline total system power: 4,100 W;
- candidate total system power: 3,000 W.

The candidate therefore has 50% lower per-GPU power but 26.8% lower modelled total-system power once the integer GPU expansion and declared overhead are included. These figures are synthetic arithmetic fixtures, not hardware measurements or an extrapolation of the Joule Point paper.

With unknown support or fixed overhead, the evaluator still reports GPU-board power but blocks the total-system power and energy-per-work claims.

## 5. v0.1 implementation

The new Rust module \`tools/measurement-audit/src/fleet_capacity.rs\` provides:
- \`evaluate_fleet_capacity_sweep\`;
- matched workload, request-mix, quality, accelerator SKU, software, boundary, demand, duration, headroom, installed-count and initial-storage validation;
- a fixed-fleet feasibility result that keeps capacity, useful-work completion and QoS gates separate;
- an integer capacity-scaled projection using measured per-GPU goodput;
- separate GPU-board and total-system power/energy;
- storage correction at the total-system boundary;
- optional sourced capex;
- baseline-relative per-GPU power/goodput, integer GPU-count delta, scaled GPU-board power, total-system power and energy-per-work changes.

Total-system deltas and Pareto comparisons are suppressed when the relevant overhead evidence classes are not comparable. The output preserves the measured/modelled/unknown overhead status.
- a candidate Pareto frontier over capacity-scaled total-system energy/work, total power, GPU count and comparable capex.

Unknown overhead blocks a total-system comparison rather than being treated as zero. Unknown capex is excluded from the cost dimension rather than being assigned a zero value.

Twelve tests are authored for the 1.25x GPU-count effect of a 20% goodput decline, fixed-fleet infeasibility versus capacity-scaled feasibility, GPU versus total-system power, unknown overhead, storage depletion, QoS/SLO regression, unknown/supplied capex, matched-contract rejection, and baseline/overhead validation. **Tests have not been executed.**

## 6. Industry comparison and next improvements

ELF and the Joule Point demonstrate that an optimal per-GPU operating point can save energy per inference while increasing latency and installed capacity. Luminous should not claim this insight as new. The additional benchmark opportunity is to make the entire trade-off reproducible across:
- fixed existing fleet versus capacity expansion;
- integer GPU count and explicit reserve capacity;
- board energy versus server/rack/facility energy;
- service quality and accepted work;
- storage depletion, support overhead and rebound;
- sourced capital cost and lifecycle replacement, when available.

### Proposed internal research gates

These are research goals, not standards or demonstrated Luminous performance:
1. At least 5% lower total-system energy per accepted useful-work unit at identical QoS.
2. Preserve offered goodput and P99 latency/TTFT contract in both fixed-fleet or explicitly capacity-scaled scenario.
3. Report fixed-fleet and capacity-scaled results separately.
4. Publish the overhead evidence class and total boundary; never call a GPU-only result a facility saving.
5. Keep integer GPU counts and reserve headroom explicit.
6. Report capex/capacity only where an auditable price source and currency basis exist.
7. Validate the capacity-scaled linear model against a physical multi-GPU replay before claiming prediction accuracy.

## 7. Future work

1. Inspect ELF's actual \`elf_master.csv\`, dictionary and power-cap trace files under its CC BY 4.0 terms; do not infer transient bandwidth from the "high-rate" label.
2. Build a source-specific ELF adapter preserving its \`settle_filter.py\` semantics and all raw windows.
3. Test the per-GPU-to-fleet projection on two or more physical GPU counts, not only one-GPU measurements.
4. Add queueing/scheduling simulation, heterogeneous GPU pools, request deadlines, redundancy and failure reserve.
5. Add measured CPU/DRAM/network/cooling/power conversion overhead and total rack/PCC traces.
6. Extend beyond inference to training, checkpoint/restart, collective communication, and mixed serving/training fleets.

## 8. Safety and claim ceiling

- This evaluator is offline and read-only. It does not control GPU power limits, clocks, requests, storage, converters or grid dispatch.
- Capacity-scaled results are linear-goodput projections, not measured fleet operation.
- Total-system claims require explicit measured/modelled overhead; board-only results are never relabelled as PCC/facility results.
- Cost claims require a cost-source reference and currency; unknown capital cost is not zero.
- No grid-service eligibility, product qualification, 800 V energization or high-energy testing is authorized.

## Sources

- Apartsin and Aperstein, [ELF: Energy-Latency Frontier dataset for GPU inference](https://doi.org/10.5281/zenodo.23074732), Zenodo, October 2026; catalogue description in the [NLM Dataset Catalog](https://datasetcatalog.nlm.nih.gov/dataset?q=0003346687).
- Apartsin and Aperstein, [The Joule Point: an Energy-Optimal Operating Point for AI Inference](https://arxiv.org/abs/2609.27926), arXiv:2609.27926, 23 August 2026.
- [ELF licence: CC BY 4.0](https://creativecommons.org/licenses/by/4.0/).
