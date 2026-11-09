# Luminous Device Workload Benchmark Programme 0.1

**Status:** Proposed benchmark protocol; thresholds are internal targets, not measured results  
**Date:** 2026-11-09  
**Parent:** [Cross-form-factor device family RFC #349](https://github.com/Luminous-Dynamics/luminous-platform/issues/349)  
**Architecture:** [Luminous device family architecture 0.1](../../docs/product/luminous-device-family-architecture-0.1.md)  
**Portfolio assurance:** [OMSI industry-profile programme #154](https://github.com/Luminous-Dynamics/luminous-platform/issues/154)

## 1. Purpose

Build a reproducible, cross-form-factor benchmark programme that answers:

1. Which device completes a real Luminous workload correctly and within its operational constraints?
2. What are latency, throughput, energy, thermal, memory and serviceability costs?
3. Which accelerator actually executes the workload?
4. Does local execution outperform an authorized remote option after network, privacy and energy costs?
5. Are results reproducible on the exact hardware, firmware, OS, toolchain, model and dataset?
6. Does the product improve the user's total cost and operational availability—not just a synthetic benchmark score?

This is not a single leaderboard or a new general benchmark framework. It composes public industry suites with profile-specific tests and the existing OMSI admission/evidence contracts. Unlike device classes and unlike workload families must not be collapsed into one weighted score.

The Platform programme owns product-level selection and comparison policy, not a second canonical workload suite, architecture optimizer or accelerator implementation. The canonical work is distributed across:

- Symthaea [HDC/CfC workload and conventional baseline map #5805](https://github.com/Luminous-Dynamics/symthaea/issues/5805), [mobile device qualification #8413](https://github.com/Luminous-Dynamics/symthaea/issues/8413), [laptop co-design #8446](https://github.com/Luminous-Dynamics/symthaea/issues/8446), [compute architecture frontier #7905](https://github.com/Luminous-Dynamics/symthaea/issues/7905), [phone/laptop co-design #8459](https://github.com/Luminous-Dynamics/symthaea/issues/8459), [GPU Fabric #6968](https://github.com/Luminous-Dynamics/symthaea/issues/6968), and [MLPerf Tiny v1.4 track #9647](https://github.com/Luminous-Dynamics/symthaea/issues/9647).
- Symtropy [laptop power/thermal twin #2697](https://github.com/Luminous-Dynamics/symtropy/issues/2697) and [phone/laptop system DSE #2706](https://github.com/Luminous-Dynamics/symtropy/issues/2706).
- Symthaea [cross-paradigm compute frontier #9236](https://github.com/Luminous-Dynamics/symthaea/issues/9236).

Use these owners' frozen task identities, input fixtures, correctness oracles and domain-specific gates. This benchmark document only defines how the Platform compares resulting evidence across product profiles without flattening it.

## 2. Reference benchmark families

Pin the exact edition, rules, harness, workload and result submission rules before each comparison.

| Domain | External reference | What it measures | Limits / Luminous use |
|---|---|---|---|
| CPU and memory | SPEC CPU 2026 | CPU-intensive workloads, memory hierarchy and compiler behavior across rate/speed suites | Not a full client, GPU, I/O, battery or system-power benchmark. Use the exact official run rules and report the suite edition. |
| AI inference | MLPerf Inference v6.1 | Standardized inference performance across datacenter, edge and supported tasks | Compare only matching scenarios, model/task and quality constraints. Do not compare unrelated systems or infer full device value from one workload. |
| Mobile AI | MLPerf Mobile v6.0 | Standardized on-device Android LLM tests (Llama 3.2 1B/3B and Llama 3.1 8B) plus vision/image workloads | Use supported exact device/backend paths; do not assume all phones or NPUs are covered. For Luminous mobile power/privacy work also compose Symthaea #8413. |
| AI PC / client | MLPerf Client v2.0 | On-device AI PC workloads, including agentic software engineering and data-analysis tasks | Use its native scenarios and scoring; add Luminous workloads separately rather than changing official tasks. |
| Embedded / MCU AI | MLPerf Tiny v1.4 | Fixed task/model/dataset/accuracy targets, latency and energy per inference for low-power devices | Reuse Closed Division for matched comparisons and Open Division for candidate innovation; do not mix the two. Compose Symthaea #9647. |\n| Energy | MLPerf Power methodology plus calibrated wall/battery measurement | Energy-aware inference measurement | Use one power boundary and disclose whether it includes display, host, accelerator, PSU losses, network and cooling. Legacy SPECpower_ssj2008 may be informative for historical server comparisons but is not a sufficient modern whole-system energy score by itself. |
| XR runtime | OpenXR Conformance Test Suite | OpenXR API/runtime conformance | Necessary for a conformant runtime, not evidence of tracking accuracy, motion-to-photon latency, comfort, optics, accessibility or safety. Those require separate measurements. |
| Storage / I/O / network | Pinned fio, iperf3 and/or applicable vendor-native tools with declared profiles | Storage and network behavior | Results depend on block size, queue depth, cache, device state, topology, link speed and test duration. No unqualified cross-vendor aggregate. |
| Environmental | Exact methods selected from IEC 60529, tailored MIL-STD-810H methods and applicable product standards | Ingress, shock, vibration, temperature and other declared environments | A method is not a universal certification. Publish tailoring, samples, orientation, preconditioning, pass criteria, instrumentation and all failures. |

### 2.1 Current release interpretation (checked 2026-11-09)

Do not treat the benchmark families as interchangeable:

- **SPEC CPU 2026** is the CPU/memory/compiler reference. It contains 52 benchmarks in SPECspeed Integer/FP and SPECrate Integer/FP suites, with optional energy reporting. It is a licensed commercial suite; check current licensing/pricing before procurement. It is not a free cross-platform whole-device battery benchmark.
- **MLPerf Inference v6.1** is divided into Datacenter and Edge suites. The September 2026 round included 10 Datacenter and 6 Edge benchmarks, including new End-to-End RAG and Agentic Edge Inference tracks. Use these for supported Node/edge profiles and only where exact task, model, scenario and scoring rules match.
- **MLPerf Client v2.0** adds end-to-end agentic Software Engineering and Data Analyst workloads that include tool execution time, plus an experimental Flux 2 Klein 4B image-generation category. Its mandatory base LLM set includes Phi 4 Mini Instruct, with Qwen 3 8B experimental. It is the strongest listed external reference for supported desktop/laptop AI-PC experiences.
- **MLPerf Mobile v6.0** brings standard LLM testing to Android devices, with Llama 3.2 1B/3B and Llama 3.1 8B plus supported CPU/NPU paths. The NPU-accelerated 8B path is device-specific; do not generalize it to every phone or NPU.
- **MLPerf Tiny v1.4** has five embedded task families: Keyword Spotting, Visual Wake Words, Image Classification, Anomaly Detection and Streaming Wake Word. Keep Closed Division reference reproduction separate from Open Division candidate innovation.
- **OpenXR CTS** establishes API/runtime conformance only. XR system performance and safety require independent optical, tracking, latency, human-factors and environmental measurements.

Use the native suite and preserve its official scoring; use Luminous frozen workloads as an additional, separate product-level evaluation. The suite's result population or headline scores are not themselves a baseline for a different device class.

Standards and upstream references:

- SPEC CPU 2026: https://open.spec.org/cpu2026/
- MLPerf Inference v6.1 official results (16 Sep 2026): https://mlcommons.org/2026/09/mlperf-inference-v6-1-results/
- MLPerf Inference v6.1 industry analysis (17 Sep 2026): https://mlcommons.org/2026/09/chairs-mlperf-inference-v6-1/
- MLPerf Client v2.0: https://mlcommons.org/benchmarks/client/
- MLPerf Mobile v6.0 results: https://mlcommons.org/2026/06/mlperf-mobile-v6/
- MLPerf Tiny v1.4 results: https://mlcommons.org/2026/07/mlperf-tiny-v1-4-results/
- OpenXR CTS: https://github.com/KhronosGroup/OpenXR-CTS
- fio: https://github.com/axboe/fio
- iperf3: https://github.com/esnet/iperf
- IEC 60529 catalogue: https://www.iec.ch/
- MIL-STD-810H official DLA record: https://quicksearch.dla.mil/qaDocDetails.aspx?ident_number=35978

Do not treat an external benchmark's existence as evidence Luminous has run it. Preserve official score rules and label independent custom measurements separately.

The current named product comparator and market baseline list is maintained in [Device Industry Reference Snapshot 2026-11](../../docs/product/device-industry-reference-snapshot-2026-11.md). It is a dated procurement research input, not a performance result; vendor claims must be independently matched before comparison.

## 3. Frozen Luminous workload suite

The Luminous suite should measure tasks the platform actually performs, not optimize a processor-specific microbenchmark that has no product relevance.

Each workload has a frozen input corpus, output/reference oracle, versioned acceptance criteria and exact runner revision. Maintain separate quality and systems tracks.

### 3.1 Symthaea cognitive workloads\n\nThe list below is a product-family projection of the canonical workload owners. Use the existing HDC/CfC workload map #5805 and the phone/laptop physical protocols #8413/#8446; add no duplicate operation semantics, fixture registry or reference implementation here.

Use the same semantic task and expected outputs across CPU, GPU and NPU backends where implementations exist:

- HDC encode, bind, bundle, similarity and associative retrieval at declared dimensions and vector precision.
- CfC/LTC recurrent sequence execution at frozen sequence lengths, state sizes and update intervals.
- Memory indexing, retrieval and context assembly under declared corpus size and concurrent load.
- Active-inference / planning decision workloads with a fixed environment and independently checked task outcome.
- Perception and multimodal workloads only where exact input data, model, preprocessing and output oracle are frozen.
- Mixed-workload interactive session: perception/retrieval/reasoning under a bounded latency and memory envelope.

Record the actual backend for every operation. If NPU support is absent, unsupported, or silently falls back to CPU, report that fact. A device's NPU TOPS specification is metadata, not a benchmark result.

### 3.2 Mycelix and platform workloads\n\nThese are platform-level measurements and must use the owning repository's exact identity, authorization, receipt, and transport fixtures. The Platform suite does not implement parallel cryptographic or authorization logic.

- Local identity / signature verification and receipt validation, with exact cryptographic suite and key sizes.
- Authorization decision fast path under single-thread and concurrent load; include p50/p95/p99 and contention.
- Offline receipt creation, verification, replay rejection and reconciliation.
- Device inventory refresh, capability-profile resolution and exact-version lookup.
- Xenia peer communication under controlled packet loss, latency, peer churn and partition.
- Nixward read-only observation, plan/validation and postcondition-check costs. Mutation paths are evaluated separately in an isolated disposable lab; no benchmark grants authority.

### 3.3 Symtropy and Sol-Atlas workloads\n\nSymtropy's existing phone/laptop DSE #2706 and thermal twin #2697 own their models, parameter spaces, calibration and prediction-error gates. The Platform consumes their qualified outputs and reports evidence level; it does not train a second general thermal model or optimizer.

- Symtropy: deterministic simulation and replay workloads with frozen initial state, solver version, timestep and tolerance; report numerical error and stability along with speed.
- Symtropy: telemetry ingestion/aggregation and model-to-observation comparison, distinguishing measured from simulated values.
- Sol-Atlas: spatial indexing, scene/map query, geometry operations and rendering/data preparation with frozen scenes and ground-truth references.
- Sol-Atlas: mapping/localization workloads only where reference trajectory/geometry and calibration are available; report drift/error, not just frames per second.

A faster approximate answer does not win if it violates correctness, uncertainty or safety acceptance criteria.

## 4. Measurement contract

### 4.1 Required manifest

Every run records:

- Unique run ID and UTC wall-clock start/end; monotonic timing source and clock quality.
- Repository URL, immutable source commit and clean/dirty state for benchmark suite and runner.
- Exact SKU, board revision, CPU stepping, GPU/NPU model, memory configuration, storage model and peripheral configuration.
- BIOS/UEFI, boot firmware, secure-boot state, microcode, EC, device firmware, OS image, kernel, drivers and accelerator runtime.
- Compiler, linker, build flags, dependency lockfiles, runtime versions and environment variables affecting execution.
- Model identifier and byte digest; quantization, precision, tokenizer, context/input length, batch size and generation/evaluation settings.
- Dataset/corpus revision, split, license, preprocessing digest and reference-oracle revision.
- Power mode, performance governor, battery state, charger/PSU, display brightness, network state, fan profile and cooling configuration.
- Ambient temperature and humidity where relevant; warm-up and thermal-equilibrium procedure.
- Instrument model, serial/identifier, calibration date/status, sampling rate, measurement boundary and uncertainty.
- Trial count, raw samples, exclusions with reasons, run failures, thermal throttling and restarts.
- Evidence artifacts and their cryptographic digests. Keep private or sensitive raw data in controlled storage; publish only permitted redacted artifacts.

No mutable branch name alone is sufficient source identity. Pin exact revisions and preserve the run manifest with the result.

### 4.2 Required measurements

Report at minimum:

- Correct/completed tasks per second and quality/acceptance rate.
- Time-to-first-useful-result, p50, p95 and p99 latency where sample counts support those percentiles.
- Throughput under specified concurrency and the tail-latency penalty.
- Joules per accepted task, joules per output unit where meaningful, idle watts and sustained watts.
- End-to-end energy boundary, not just accelerator-board telemetry.
- Peak and steady-state memory; memory bandwidth only when measured with a declared method.
- Sustained clocks, power limits, thermal state, fan behavior and throttling.
- Battery drain and practical runtime under a declared display/workload profile for mobile devices.
- Error rate, task quality, uncertainty calibration and abstention quality for AI workloads.
- Network egress, transfer volume, packet loss, and local-vs-remote time/energy costs for distributed workloads.
- Repair time, parts cost, spare-part lead time, post-repair qualification time and downtime for serviceability profiles.

For a quality-sensitive workload, report energy per **correct accepted result**, not energy per token or operation alone. Do not reward a faster system that lowers answer quality.

### 4.3 Trial and uncertainty policy

- Use at least five independent measured runs for an initial screening result; use larger samples for claims with meaningful tail-latency or failure-rate requirements.
- Separate cold-start and steady-state runs.
- Reach a declared thermal condition before sustained tests; do not mix short burst and sustained results.
- Publish median and spread; use confidence intervals or bootstrap intervals where appropriate.
- Keep failed, timed-out, throttled and invalid runs in the record. Do not delete them because they make a chart less attractive.
- Repeat selected runs on another day or independent operator/device to estimate reproducibility.
- A sample size too small to support a percentile or failure-rate claim must be labelled insufficient.

## 5. Device-specific test matrix

| Family | Primary tests |
|---|---|
| Mobile | Battery runtime, idle drain, sustained local Symthaea tasks, thermal throttling, modem/Wi-Fi behavior, camera/mic privacy controls, update/recovery, screen/battery/port service |
| Book | Sustained CPU/GPU/NPU workload, battery under defined tasks, display/keyboard/port repair, fan/acoustic behavior, suspend/resume, dock/display interoperability, offline recovery |
| Desk | CPU/memory/accelerator throughput, local/remote workload placement, wall power, thermals/noise, expandability, memory/storage replacement and recovery |
| Node | Idle and loaded wall power, throughput per watt, ECC/error reporting where supported, storage resilience, network I/O, concurrent tenants, remote management, power-loss recovery and firmware rollback |
| Slate | Battery, touch/pen latency, local inference, display/thermal behavior, accessibility and replaceable service parts |
| Table | Multi-user input latency, touch accuracy across display area, concurrent sessions, accessibility/reach, thermal/noise, cleanability and field-replaceable display/compute/I/O |
| Vision | OpenXR CTS, runtime frame timing, motion-to-photon, tracking accuracy/drift against ground truth, dropped/reprojected frames, fit/comfort study, thermal/weight distribution, sensor privacy and safe fail/exit behavior |

## 6. Distributed-device fabric tests

Compare local-only, remote-only and policy-authorized hybrid execution on the same task:

1. Local device with network disabled.
2. Trusted peer in the same LAN.
3. Trusted remote node with measured WAN latency.
4. Network interruption during task execution.
5. Peer revocation or stale device evidence during a session.
6. Destination that is not authorized for the input data classification.
7. Source and destination with mismatched model/runtime/workload versions.
8. Uncertain remote acknowledgement or lost completion receipt.

Measure task completion, latency, network bytes, energy on source and destination, recovery time and evidence completeness.

Hard safety/privacy gates:

- Zero unauthorized offloads in the frozen negative-control corpus.
- Zero sensitive-data transfer to an untrusted or unapproved destination.
- Stale, revoked or mismatched identity/evidence cannot silently satisfy the trust policy.
- Network loss must not create new authority or cause the source to claim remote completion without a verified result.
- Unknown destination capability returns a bounded refusal or local fallback, never an assumed capability.

These are acceptance criteria for the test harness. They are not claims about current implementation.

## 7. Competitive comparison protocol

### 7.1 Baseline classes

Compare by workload and intended product profile, not logo:

- Current OEM consumer and professional devices in the same price/performance bracket.
- Repairability-oriented products for serviceability and lifecycle comparison.
- Rugged OEM devices for field durability and support comparison.
- Enterprise server/workstation vendors for sustained compute, manageability and power.
- Mobile/XR reference systems for battery, thermal, latency, runtime compatibility and ergonomics.
- Existing deployment, device management, firmware, security and AI software stacks for the software lifecycle workflow.

A product may be better in one dimension and worse in another. Show the Pareto frontier rather than a single winner. Cost comparisons must use the same currency, region, tax/import basis, warranty period, support level, memory/storage configuration and lifecycle assumptions.

### 7.2 Matched comparison rules

Two runs are comparable only if the task, dataset/split, model/quality, output acceptance, runtime constraints, concurrency, power/thermal conditions and measurement boundary are equivalent or a documented normalization is justified.

Publish the full configuration and resource budget, including purchase price, power and cooling. Compare both official benchmark results (under official rules) and Luminous product workloads; do not rewrite an external benchmark's rules to favour Luminous.

If results are not matched, label not_comparable and explain why. Do not impute missing values or extrapolate results across SKUs.

### 7.3 Proposed internal stretch targets\n\nThese cross-family targets do not override stricter profile-specific proposals in the canonical owners (for example, mobile #8413 and accelerator selection gates in #8459/#7905). Always report the owner, exact workload and target version.

These are **targets to attempt**, not achievements or external SOTA claims:

- Demonstrate at least a 10% improvement in energy per correct accepted Symthaea task or p95 latency versus the strongest matched baseline, while maintaining or improving task quality and staying within the same declared power/thermal envelope.
- Demonstrate at least a 10% improvement in sustained useful throughput per watt on one selected workload/profile without increasing error or abstention failure rates.
- Demonstrate at least a 20% reduction in five-year cost per verified operating hour versus a matched baseline for one serviceable field/enterprise profile, including purchase, energy, support, downtime, parts and repair.
- Demonstrate repair-triggered qualification that prevents unsupported post-repair ingress/durability claims and is independently verifiable offline.
- Achieve zero unauthorized effects and zero unauthorized sensitive-data offloads across the complete frozen negative-control suite.

An improvement target is only met when the quality, cost, security and reliability constraints are satisfied together. A safety failure cannot be compensated by a performance gain. If no baseline is measured, report the target as unassessed.

## 8. Evidence maturity and publication

Use the Luminous evidence vocabulary:

- conceptual
- specified
- implemented
- structurally_checked
- ci_tested
- bench_measured
- physically_qualified
- independently_reproduced
- field_validated

Every public result should include the exact source commit, run-manifest digest, artifact digests, benchmark suite revision, configuration, raw/derived data policy, result, failures, limitations, reviewer identity/role and reproduction instructions.

A signed receipt proves that an identified signer made a statement over particular bytes; it does not make the measurement true. A passing schema, VM simulation or CI job does not prove physical hardware performance. An exact-head CI result and a physical measurement are different evidence classes.

## 9. First baseline campaign

### Step A — Inventory without mutation

- Capture exact CPU, GPU, NPU visibility, memory, storage, board/system identity, firmware, kernel/driver and power source.
- Do not change firmware, boot settings, governor, kernel or system configuration during baseline discovery.
- Redact serials, keys, hostnames and user data from public artifacts.

### Step B — Build the minimum workload set

Choose three frozen Symthaea workloads with clear oracles: one HDC retrieval, one CfC/LTC recurrent workload and one complete user-facing task. Include CPU as the common baseline. Add GPU/NPU only if exact backend support is verified.

### Step C — Measure whole-system cost

Capture wall power at the AC input for desktop/laptop systems where a suitable meter is available. For battery devices, record battery telemetry and validate it against a known procedure; report telemetry limitations. Separate display, networking and idle conditions.

### Step D — Publish negative and positive runs

Retain cold-start, throttled, failed and incomplete results. If the device cannot access an NPU, report not_available rather than silently claiming accelerator performance.

### Step E — Reproduce

Have a second run/operator repeat the frozen set with the same exact source and manifest. Only then compare external baseline systems.

## 10. Claim ceiling

This protocol and its target values do not establish any result. No better-than-SOTA, lower-TCO, ruggedness, privacy, performance-per-watt, safety or hardware-support claim is permitted until the specific target, benchmark rules, sample size, exact equipment and independent evidence support it.
