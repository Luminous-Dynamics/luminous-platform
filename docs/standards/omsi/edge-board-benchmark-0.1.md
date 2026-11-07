# OMSI Edge Intelligence Board Benchmark 0.1

**Owner:** Luminous Platform / OMSI EDGE-SENSOR-001 [#300](https://github.com/Luminous-Dynamics/luminous-platform/issues/300)  
**Status:** Design and machine-readable qualification manifest; no board selected or tested.

## Purpose

Convert the ESP32-S3 versus ESP32-P4 discussion into a reproducible, exact-board qualification exercise. The chip-family name is not a hardware identity. No candidate is selected until the actual development board, module, silicon revision, memory configuration, connectivity companion, schematic/BOM and firmware toolchain are pinned.

## Current official vendor context (checked 2026-11-07)

- ESP32-P4 current series datasheet is preliminary pre-release v0.7 (revision history 14 July 2026), describing a dual-core high-performance RISC-V processor, separate single-core low-power processor, 768 KB HP L2 memory, 32 KB LP SRAM, 8 KB scratchpad and in-package 16/32 MB PSRAM variants. A preliminary datasheet is not final board qualification. https://documentation.espressif.com/esp32-p4_datasheet_en.html
- ESP-DL v3.3.14 (19 October 2026) reports >3× speedup on selected MobileNetV2/YOLO11 workloads on S3 and >1.9× on P4 compared with earlier ESP-DL releases. These are selected model/library-version comparisons, not an S3-vs-P4 benchmark, competitor comparison, or evidence for our sensor workload. https://github.com/espressif/esp-dl/releases
- ESP-DL supports mixed W8A16 quantization and target-specific kernels. Record the exact runtime release, quantization mode, operator support/fallback map and compiler flags. Compare accuracy, calibration and abstention on identical frozen windows. https://docs.espressif.com/projects/esp-dl/en/latest/tutorials/how_to_quantize_model.html
- Espressif's ESP-IDF guidance treats secure boot, flash encryption, signed OTA, rollback and anti-rollback as distinct controls. Flash encryption alone is not a substitute for secure boot. Exact target/configuration and irreversible eFuse state must be recorded. https://docs.espressif.com/projects/esp-idf/en/stable/esp32p4/security/security.html and https://docs.espressif.com/projects/esp-idf/en/stable/esp32s3/security/security.html

## Qualification unit

Freeze this complete identity for every measurement:

`board manufacturer + model + PCB revision + module part + SoC silicon revision + flash/PSRAM + connectivity companion + BOM/schematic digest + sensor part/revision + ESP-IDF + ESP-DL/TFLM + compiler + build flags + operator map + quantization + model digest + input corpus digest`.

The current manifest deliberately marks both candidates `UNSELECTED_EXACT_BOARD_REQUIRED`; missing identity is not guessed. ESP32-P4 wireless connectivity is represented as a companion requirement until an exact board/module is selected.

## Matched workload and baselines

Use one frozen non-transmitting sensor-feature workload and identical input-window bytes/labels for all arms. The starter synthetic window is 5 seconds at 100 feature windows/s across eight declared channels. It is a deterministic software workload, not a claim that every physical sensor should be sampled at that common rate.

Compare fixed threshold/statistical detection, host FP32 reference, ESP-DL quantized inference on each exact target, TFLite Micro only when its operator/target support is verified, and optional cloud-assisted operation with network partitions included.

## Metrics

- Correctness: PR-AUC, macro-F1, confusion matrix and false alerts per operating day.
- Calibration/epistemics: Brier/log loss, calibration curves, selective risk/coverage, abstention correctness and OOD false acceptance.
- Runtime: P50/P95/P99 latency, throughput and deadline-miss rate.
- Memory/storage: peak internal RAM, PSRAM, stack, static/heap allocation, model/feature/firmware flash footprint.
- Energy/thermal: energy per inference/window, idle and active current, peak power, die/board temperature and throttling.
- Acquisition integrity: dropped samples, overrun/underrun, ring-buffer high-water mark, timestamp discontinuity and unreported loss (must be zero).
- Offline/privacy: local function through WAN/DNS/platform outages, bounded retention, explicit raw-data export policy and exfiltration negative controls.
- Security/recovery: signed boot/update, flash protection, rollback, anti-rollback, debug-interface policy, power-loss recovery and independently verified postconditions.
- Evidence: exact build/input/model/profile digest and independent offline reconstruction.

No aggregate score may let speed or accuracy compensate for unreported sample loss, unauthorized export, invalid measurement admission or failed recovery.

## Required comparisons and controls

1. Float32 host reference versus each board's quantized implementation on identical windows.
2. Per-operator and per-layer comparison, including fallback operators and target-specific rounding/quantization behavior documented for the exact runtime.
3. Quantization threshold-flip fixtures: a model whose output moves across an operationally relevant classification threshold must be counted as a regression, not averaged away.
4. OOD and acquisition/measurement invalidity controls: a high model score cannot override missing calibration, sensor saturation, clock reset or stale input.
5. Power-loss during update, stale/unsigned firmware, rollback/anti-rollback, ring-buffer overflow and network partition.
6. Same physical board revision tested across repeat runs and temperature/power conditions; distinguish within-board reproducibility from cross-board transfer.

## Files

- `edge-board-benchmark-manifest-0.1.schema.json`: strict Draft 2020-12 schema.
- `edge-board-benchmark-manifest-0.1.json`: two unselected candidate slots, exact workload definition, comparator and metric tracks, 12 deterministic scenario definitions, and an empty run-record list.

## Claim ceiling

This manifest does not select or qualify any board, model or sensor. No latency, energy, calibration, security, radio compliance, PNT integrity, physical truth or production readiness is claimed. Physical measurements must be added as exact run records and independently verified before any comparison or board selection.
