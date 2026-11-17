# Kitchen Workload Boundary V0.1

**Status:** design contract only; not a runnable workload profile, device adapter, hardware qualification or actuation interface.

**Tracks:** [Luminous Platform #487](https://github.com/Luminous-Dynamics/luminous-platform/issues/487), [Symtropy KITCHEN-SYSTEMS-001 #2313](https://github.com/Luminous-Dynamics/symtropy/issues/2313), [Symtropy KITCHEN-WORKCELL-001 #5618](https://github.com/Luminous-Dynamics/symtropy/issues/5618).

## 1. Purpose and non-goals

Define how a local-first kitchen service can be supervised on a Luminous host without turning the Platform into a second recipe planner, cognitive agent, appliance controller, food-safety authority or marketplace.

V0.1 establishes a **read-only workload boundary**. It does not connect to, discover, configure or command a physical appliance. The existing generic [Workload Profile v1alpha1](../schemas/workload-profile.schema.json) is the deployment-policy starting point; a future typed adapter/device profile needs its own schema and threat review before any hardware access.

The governing invariant remains: **Platform components may observe host state; they must never be required for host boot.**

## 2. System ownership

| System | Owns | Must not own |
|---|---|---|
| Luminous Platform | workload lifecycle, NixOS/OCI/VM placement, host privilege boundary, secrets, signed updates, local storage policy, service health, backup/restore and operator lifecycle receipts | cooking plans, recipe truth, device control, food safety or producer provenance |
| Symthaea | user-directed culinary reasoning, preference-aware options, recipe adaptation, uncertainty and bounded workflow proposals | appliance safety certification, physical state truth, silent actuation authority |
| Symtropy | typed recipe/action graph, deterministic scheduling, exact task-to-module capability admission, simulation/measurement separation, resource accounting and reproducible evaluation | generic cross-vendor command channel, certified interlocks or safety claims |
| Mycelix | producer/product/lot identity, custody and handling evidence, food/packaging lifecycle and exchange | home appliance control or unverified inventory claims |
| Sol Atlas | regional climate, water, supply and food-resilience context with explicit uncertainty | plot-level yield or household safety claims without observations |
| Appliance-local controller | certified or otherwise independently qualified interlocks, temperature/pressure limits, motor/electrical protection and emergency stop | authority delegated to a remote cognitive system |

No upstream system may promote a modelled or declared capability into a measured or independently qualified fact.

## 3. V0.1 workload posture

The first deployment candidate is an isolated, unprivileged service on a NixOS host, but the profile remains **design-only** until a separate PR provides the exact immutable image/derivation and test receipts.

Required default policy:

- non-root UID/GID; no privileged container; no Linux capabilities;
- read-only root filesystem and `noNewPrivileges`;
- no host PID, network or IPC namespace;
- no host control and no broad host data access;
- no inbound listener; egress denied unless a specific, documented local dependency is approved;
- no secrets embedded in images, fixtures or logs;
- immutable image/derivation digest and versioned declarative configuration;
- manual or on-demand start; no boot dependency; changes require explicit authorization;
- CPU, memory, disk and process limits; bounded logs and retention;
- local export and deletion paths that do not require a cloud account.

This profile does not imply that rootless containers alone are a sufficient security boundary. Use the existing platform workload policy, namespace/LSM/seccomp controls and VM placement according to the threat model. Do not grant a general-purpose cognitive agent Podman, libvirt, Incus or host service-manager administration.

## 4. Device and adapter boundary

V0.1 has **no direct device access**. Any future read-only adapter needs a separate versioned profile that binds:

- vendor, product/model, hardware revision, region/market SKU and firmware revision;
- device identity method and a privacy-preserving local identifier;
- protocol and adapter implementation digest;
- exact read-only fields, units, sampling intervals and data-quality states;
- permitted device nodes or transport endpoints, if any, each individually allowlisted;
- calibration/evidence references and the scope of each declared observation;
- timeout, rate, message-size and parser-resource limits;
- update/revocation policy and operator-visible health state.

Unknown model/firmware, stale or duplicate identity, unsupported protocol, malformed frame, replay, clock discontinuity, missing calibration or revoked adapter must yield an unavailable/indeterminate observation. The adapter must not guess, substitute another device revision or silently fall back to broader permissions.

**No generic actuator API.** The workload must not expose arbitrary shell, generic RPC, motor, blade, heat, pressure, valve, lid-lock or dosing commands. Any later command path requires a separate per-device assurance case and review; appliance-local interlocks remain authoritative. A Symthaea proposal, Symtropy schedule, Platform lifecycle receipt or Mycelix credential is not an actuation permit.

If an implementation needs host-side IPC or a device node, the current generic Workload Profile is not sufficient by itself. Extend it with a typed, read-only, exact-identity mount/device contract and negative tests; do not smuggle access through an untyped writable mount or unrestricted network rule.

## 5. Observation and evidence model

Every observation should retain the distinction between:

- `declared`: asserted by a vendor or profile;
- `modelled`: derived from a specified model and assumptions;
- `simulated`: generated by a named, versioned simulator;
- `measured`: observed with instrument identity, calibration and method references;
- `independently_qualified`: reviewed/tested by an appropriately competent independent party for a clearly bounded claim.

A future observation receipt should bind at minimum:

1. event and device-local sequence identity;
2. host identity, workload digest and configuration digest;
3. adapter digest and exact device/model/firmware identity;
4. monotonic timestamp plus wall-clock timestamp and clock-quality status;
5. field, value, unit, source, evidence class and quality/status;
6. calibration, test-method and raw-artifact digest references where applicable;
7. consent, retention and export classification.

Missing, stale, contradictory or out-of-range observations remain explicit. Missing energy, water, ingredient mass or temperature is never silently converted to zero. A digest proves identity of bytes, not truth of the observation.

## 6. Offline and failure behavior

- Local planning, manual operation and export must remain usable without WAN, vendor cloud or Luminous account.
- Loss of a telemetry source does not imply the appliance is off, safe, empty, clean or available.
- An unknown device state is shown as unknown. Do not retry an ambiguous mutation because V0.1 has no mutation path; any later command design must reconcile before repeating.
- Adapter crash or restart must not affect host boot, appliance-local operation or other appliances.
- Stale firmware, revoked identity, parser error, sequence rollback or resource exhaustion quarantines that adapter only.
- Update rollback returns to the last known workload/configuration digest; a restart is not proof of restored physical state.
- Appliance-local physical controls remain usable even when the Platform or household network is unavailable.

## 7. Required threat cases

At minimum, test and retain receipts for:

- compromised adapter image or dependency;
- device identity spoof, cross-device mix-up and stale firmware;
- replayed, reordered, duplicated or truncated telemetry;
- parser fuzzing, oversized payloads, invalid units and numeric overflow;
- clock rollback, sequence reset and host reboot;
- unexpected network egress and attempted ingress;
- forbidden device-node, mount, namespace, capability and host-control access;
- data retention leak, overcollection and unintended household disclosure;
- update interruption, wrong digest, rollback and partial state;
- host shutdown/boot with service disabled, failed, corrupted or unavailable.

The first VM test must prove that disabling, corrupting or removing the kitchen workload cannot prevent the host from booting or mutate the base NixOS configuration.

## 8. Interface to the other kitchen repositories

- Symthaea can propose a meal/workflow request with user-owned constraints, source refs, uncertainty and explicit abstentions.
- Symtropy validates recipe graphs, task schedules, module bindings and declared household/module power envelopes. The draft [capability-admission PR #5863](https://github.com/Luminous-Dynamics/symtropy/pull/5863) is not yet merged or physically qualified.
- Mycelix can contribute product/lot and packaging events with explicit custody and availability status. A forecast is not stock; a lot claim is not a safety certificate.
- Platform stores/supervises signed workload and event artifacts under local retention/consent rules, but it must not rewrite the source domain's claims or turn them into permissions.

Prefer stable, versioned, content-addressed event contracts over a shared mutable database. Each subsystem remains independently operable and replaceable.

## 9. Acceptance gates

V0.1 design can advance only when:

1. the profile passes the existing Workload Profile schema and its policy validator;
2. image/derivation, lockfile, configuration and receipts bind exact immutable digests;
3. NixOS VM tests pass for privilege denial, read-only filesystem, host namespace isolation, network policy and bounded resources;
4. host-boot independence is tested with the workload disabled and corrupted;
5. offline restart, update interruption, rollback and local export are tested;
6. the threat cases above have explicit attempted/pass/fail/skipped records;
7. unsupported and missing device facts remain unavailable, never inferred.

No physical adapter is enabled at this gate.

## 10. Claim ceiling

A passing V0.1 workload test would establish only that one exact workload configuration was packaged and exercised within a tested host isolation/lifecycle boundary. It would not establish device compatibility, telemetry truth, cybersecurity against every malicious actor, food safety, appliance safety, energy/water savings, repairability or kitchen-system superiority.
