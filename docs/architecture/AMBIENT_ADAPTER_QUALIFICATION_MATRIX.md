# Ambient Adapter Qualification Matrix

**Status:** source-inspection map; no adapter is runtime-qualified by this document.

## Scope and evidence rules

A renderer core, manifest parser, host adapter, and displayed product are different qualification subjects. Do not transfer a capability from one layer to another. The following map is tied to immutable Sovereign Boot source snapshot [`ab6eac9641e0673cf2de36018a35986c122b36a8`](https://github.com/Luminous-Dynamics/sovereign-boot/commit/ab6eac9641e0673cf2de36018a35986c122b36a8). The upstream branch has moved since that snapshot; a comparison found newer commits adding contract/CI/evidence-harness work without modifying the adapter implementation files listed below.

## Adapter matrix

| Adapter | Source entrypoints at the pinned snapshot | Manifest / settings boundary | Source-inspected behavior | Explicit non-claims |
|---|---|---|---|---|
| Browser-WASM demo | [`visual-wasm/src/lib.rs`](https://github.com/Luminous-Dynamics/sovereign-boot/blob/ab6eac9641e0673cf2de36018a35986c122b36a8/crates/visual-wasm/src/lib.rs); [`www/app.js`](https://github.com/Luminous-Dynamics/sovereign-boot/blob/ab6eac9641e0673cf2de36018a35986c122b36a8/crates/visual-wasm/www/app.js) | Calls the strict Scene Pack parser, then accepts an explicit variant and constructs typed renderer settings. | Applies selected brightness and FPS cap; pauses animation when `document.hidden`; reduced-motion selects `staticFallback`. | Does not implement non-empty safe regions or unsupported compositions; rejects asset-bearing packs without a trusted resolver, required capabilities and inputs. The reduced-motion choice can differ from manifest `reducedMotionPresentation`. Hidden-page pause does not prove display-sleep or OS lock/suspend behavior. |
| WASI one-shot renderer | [`visual-wasi/src/main.rs`](https://github.com/Luminous-Dynamics/sovereign-boot/blob/ab6eac9641e0673cf2de36018a35986c122b36a8/crates/visual-wasi/src/main.rs) | Parses a Scene Pack and selected presentation for one deterministic image. | Can render centered-network boot or gradient-only staticFallback; applies brightness. | Rejects safe regions, unsupported compositions, assets and unsupported metadata. A one-shot image has no ongoing page-visibility or display-sleep scheduler. Do not infer lifecycle support from its parser. |
| WIT component guest | [`visual-component/src/lib.rs`](https://github.com/Luminous-Dynamics/sovereign-boot/blob/ab6eac9641e0673cf2de36018a35986c122b36a8/crates/visual-component/src/lib.rs) | Receives typed settings through WIT; the guest API is not itself a JSON Scene Pack parser. | Configures and advances the shared core, returns RGBA frames, and exposes a static-fallback operation. | No display, window, OS lifecycle, JSON loading, filesystem asset resolution, or authentication authority inside the guest. Guest capability is not host-integration qualification. |
| Wasmtime conformance host | [`component-host-smoke/src/main.rs`](https://github.com/Luminous-Dynamics/sovereign-boot/blob/ab6eac9641e0673cf2de36018a35986c122b36a8/tools/component-host-smoke/src/main.rs) | A test harness parses/maps a pinned fixture and invokes the actual WIT component ABI. | Intended to exercise component instantiation, bounded operations, determinism, resource limits and rejection behavior with no registered imports. | Test harness is not a production display host and has no compositor/lock/suspend adapter. Authored tests are not a passing exact-head run. |
| Native DRM boot renderer | [`quicken-fb/src/main.rs`](https://github.com/Luminous-Dynamics/sovereign-boot/blob/ab6eac9641e0673cf2de36018a35986c122b36a8/crates/quicken-fb/src/main.rs) | The inspected path constructs the core with a genesis phrase; it does not load Scene Pack JSON. | Deterministic phrase-seeded DRM/KMS rendering. | No claim of numeric Scene Pack seed, manifest parser, configured palette or presentation semantics on this boot path. Do not transfer browser/WASI manifest support into native boot claims. |

## Checked-in compatibility profile

The profile at [`sovereign-visual-core.source-inspection.profile.json`](../../contracts/examples/sovereign-visual-core.source-inspection.profile.json) is explicitly scoped to **browser-wasm-demo** and names the two source paths backing that scope. It declares only browser motion/composition values identified in source. It records `onHidden:pause` because the browser loop checks `document.hidden`; it does not declare display-sleep handling, OS lock behavior, suspend behavior, or wake reinitialization.

Every compatibility report includes the adapter ID and entrypoint paths so saved reports preserve their target context. Profiles for WASI, WIT, Wasmtime, and native boot need independent capability declarations before they can support adoption claims.

## Evidence gate per adapter

For each adapter, preserve the exact source commit, entrypoint paths/blob IDs, profile/schema version, contract/schema digest, fixture digest, toolchain/runtime versions, test result, and output digest. A green test on one adapter must not mark another adapter green. Minimum sequence:

1. Static source/capability review with typed unsupported-field cases.
2. Exact-head build and unit/component tests.
3. Pinned fixture to normalized effective-settings receipt.
4. Deterministic output/replay check on that adapter; cross-adapter RGBA comparison only where byte-level parity is the chosen requirement.
5. Runtime-specific resource, lifecycle, and fallback checks.
6. Evidence linked to the exact reviewed commit; queued, cancelled, or stale-head runs never qualify.

**Current state:** all rows are source-inspection only. No screenshot, benchmark, display lifecycle, lockscreen, native boot, or cross-adapter runtime claim is granted here.
