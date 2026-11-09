# Read-Only Hardware Baseline Runbook 0.1

**Status:** operator procedure; no measurements are included  
**Date:** 2026-11-09  
**Parent:** [Luminous device family RFC #349](https://github.com/Luminous-Dynamics/luminous-platform/issues/349)  
**Benchmark protocol:** [Luminous Device Workload Programme 0.1](../../benchmarks/luminous-device-workload-v0.1/README.md)  
**Cycle microbenchmark hardening:** [Symthaea #11238](https://github.com/Luminous-Dynamics/symthaea/issues/11238)

## Purpose and safety boundary

Collect a baseline of an owner-authorized Linux/NixOS or Android device before workload profiling. This procedure is observational:

- Do not change firmware, UEFI/BIOS settings, boot mode, governor, power profile, kernel, drivers, radio state, secure-boot state or installed software.
- Do not flash, unlock, root, install an APK, invoke a rebuild, or change device configuration.
- Do not run privileged collection tools merely to obtain a more complete inventory.
- Do not upload raw command output. Review and redact it locally first.
- Stop if a command unexpectedly proposes a change, requests privileged access, or affects availability.

This is an inventory snapshot, not a security appraisal, measured benchmark, thermal qualification or hardware certification.

## 1. Run identity

On the repository checkout, record:

~~~~sh
date -u +%FT%TZ
git rev-parse HEAD
git status --short
~~~~

Record the exact repository and commit manually in the local run record. A dirty checkout must be labelled dirty and its changed-file manifest retained privately. Do not use a branch name as source identity.

Record the benchmark executable's own version/build identity once the runner emits it. Until then, record the build command, toolchain version and executable digest separately.

## 2. Linux / NixOS host snapshot

Run only commands already available on the host. Missing utilities are not_available; do not install packages solely for this inventory pass.

~~~~sh
uname -a
cat /etc/os-release
lscpu
free -b
lspci -nnk
lsusb
nix --version 2>/dev/null || true
~~~~

For NVIDIA systems, request only non-unique device fields:

~~~~sh
nvidia-smi --query-gpu=name,driver_version,memory.total,power.limit --format=csv,noheader
~~~~

Do not use the default full nvidia-smi -q output for publication; it may include UUIDs, serial-like identifiers and other unique fields. Do not infer NPU availability from an unsupported or empty query.

For available sensors and battery telemetry:

~~~~sh
sensors -j 2>/dev/null || true
for p in /sys/class/power_supply/BAT*; do
  [ -d "$p" ] || continue
  printf '%s\n' "battery_class=$(basename "$p")"
  for f in type status capacity energy_now energy_full power_now voltage_now; do
    [ -r "$p/$f" ] && printf '%s=' "$f" && cat "$p/$f"
  done
done
~~~~

Battery sysfs fields are OS-exposed estimates/telemetry, not calibrated energy measurements. Sensor names and values may vary by platform; retain the exact source and field names.

### Linux fields to record

| Field | Source | Evidence class |
|---|---|---|
| CPU model, topology, ISA flags | lscpu | OS-reported inventory |
| GPU model, driver, memory | lspci, selected vendor query | OS/driver-reported inventory |
| RAM capacity and topology where exposed | free, lscpu, vendor documentation | Report source explicitly |
| Kernel / distribution | uname, /etc/os-release | OS-reported |
| Storage model/controller | lspci and available non-privileged block inventory | OS-reported; do not publish serials |
| Firmware / Secure Boot | Existing user-visible system settings or trusted inventory tool already installed | Report unknown if unavailable; do not alter settings |
| Temperature | Existing sensor interface | Telemetry, not calibrated surface temperature |
| Battery | OS/sysfs telemetry | Proxy, not calibrated joules |
| Accelerator backend | Runtime-specific execution receipt | Must be observed during workload; inventory alone is not proof of use |

Do not run dmidecode, raw ACPI dumps, complete fwupdmgr get-devices dumps or vendor debug bundles for this first baseline unless the operator reviews the exact fields locally. These may expose serials, firmware identifiers or machine-specific information.

## 3. Android device snapshot via USB ADB

Use an owner-authorized device connected by USB. Avoid wireless ADB for the initial inventory. Do not publish the output of adb devices -l, because it can include a device serial.

Read the following individually:

~~~~sh
adb shell getprop ro.product.model
adb shell getprop ro.product.device
adb shell getprop ro.build.version.release
adb shell getprop ro.build.version.sdk
adb shell getprop ro.build.version.security_patch
adb shell getprop ro.boot.verifiedbootstate
adb shell getprop ro.boot.flash.locked
adb shell getprop ro.boot.vbmeta.device_state
adb shell cat /proc/meminfo
adb shell dumpsys battery
adb shell dumpsys thermalservice
~~~~

These are device-reported properties and diagnostics. They do not independently appraise the boot chain or prove key attestation. A value such as "green" is a report from the platform; a verifier must separately validate a fresh attestation and its reference values before any higher-assurance claim.

Do not collect or publish:

- IMEI, IMSI, phone number, Android ID, serial number or advertising ID;
- Wi-Fi/Bluetooth MAC addresses, BSSID, SSID, IP addresses or network identifiers;
- account names, installed-app inventory, notification content or user files;
- raw camera, microphone, location or other personal sensor content;
- ADB device serial or full build fingerprint unless reviewed and redacted.

Do not enable USB debugging, unlock the bootloader, install a test service, change permissions, or modify the device to collect this snapshot. If ADB is not already authorized, mark the Android fields not_collected.

### Android fields to record

| Field | Source | Evidence class |
|---|---|---|
| Product model/device code | getprop | Device-reported |
| Android release/API level/security patch | getprop | Device-reported |
| Verified Boot state / lock indicators | selected boot properties | Device-reported, not independent appraisal |
| Memory | /proc/meminfo | OS-reported |
| Battery charge/status/temperature | dumpsys battery | OS telemetry, not calibrated energy |
| Thermal status/zones | dumpsys thermalservice | OS telemetry; not calibrated surface temperature |
| CPU/GPU/NPU execution | workload-specific runtime receipt | Must be observed during workload; capability alone is insufficient |

## 4. Inventory normalization

For every field, preserve:

- value and unit;
- collection method and tool/version;
- timestamp;
- evidence class (measured, vendor_declared, os_reported, inferred, simulated, unknown);
- exact device model/SKU and board revision when legitimately available;
- confidence/limitations;
- redaction status.

Never replace missing values with zero, a vendor-family average or an assumed capability. Use unknown, not_available, or not_collected with a reason.

Keep the unredacted output on the owner's device. Publish only the reviewed normalized summary and permitted artifacts.

## 5. Baseline conditions before running workloads

Record, without changing the device:

- ambient temperature and humidity, if an instrument is available;
- AC or battery power source and current charge band;
- display brightness and refresh configuration;
- connected display, dock, charger and peripherals;
- network state and link type;
- current user-selected power mode;
- device uptime and recent workload history if available;
- whether the device is idle, thermally settled, or recovering from a previous test.

Do not disable radios, change screen brightness or select a different performance mode as part of the initial inventory. Those are later controlled experiment conditions.

## 6. Workload baseline staging

The canonical workloads and acceptance oracles remain owned by Symthaea:

- HDC retrieval / similarity;
- HDC binding or bundling;
- one representative CfC/LTC state-update path;
- one complete user-facing task with a frozen correctness oracle.

Use the exact existing workload implementations and fixtures. Do not create a duplicate HDC implementation or alter task semantics just to fit a device.

Until [SOMA-CYCLE-BENCH-001 #11238](https://github.com/Luminous-Dynamics/symthaea/issues/11238) is implemented and qualified, the current soma_bench output is cycle-level exploratory timing only. It must not be presented as:

- measured host cadence or sustained Hz;
- battery life or energy per task;
- GPU/NPU acceleration;
- sustained thermal behavior;
- task success or intelligence quality.

For a physical energy result, use the preregistered #8413 protocol and calibrated instrument boundary. For laptop thermal prediction, use Symtropy #2697's held-out physical-trace protocol. Keep these evidence classes separate.

## 7. Publication checklist

Before publishing a baseline:

- [ ] Exact device identity is model/SKU scoped without exposing unique serials.
- [ ] Source repository and immutable source commit are recorded.
- [ ] Firmware, OS, kernel, driver and runtime versions are recorded or explicitly unknown.
- [ ] Collection tools and commands are listed.
- [ ] Sensitive identifiers and user data are redacted.
- [ ] Vendor claims, OS telemetry, calibrated measurements and simulation are clearly separated.
- [ ] No setting, firmware or device state was changed during inventory.
- [ ] No command output or artifact is described as a physical qualification without the corresponding test and evidence.

A complete baseline report is a starting point for controlled experiments, not a performance result.
