# Ambient Scene Pack validation and renderer compatibility

## Scene Pack schema validator

The CLI validates the manifest schema as JSON Schema Draft 2020-12, applies format checks, then checks cross-field invariants and packaged asset files. It rejects duplicate object keys, invalid UTF-8, non-standard numeric constants, unresolved local schema references, unsupported legacy recursive-reference keywords, and external or relative schema-resource loading.

Run from the repository root:

    python -m pip install -r tools/ambient_validation/requirements.txt
    python tools/ambient_validation/validate_scene_pack.py contracts/examples/first-germination.scene.json
    python tools/ambient_validation/test_validate_scene_pack.py

For asset-bearing packs, pass --asset-root with the unpacked package root. Renderer capabilities must be explicitly declared with repeated --supports-capability options; unsupported requirements fail closed.

The validator checks unique asset/input IDs, normalized safe-region containment, static fallback rules, FPS/branch budgets, explicit capabilities, package path containment, symlink escape, and SHA-256 asset integrity. It is not a filesystem sandbox against concurrent mutation. It verifies each asset's hash and per-asset license identifier, but Scene Pack v1 still lacks a separate attribution-manifest schema; full attribution-manifest completeness is therefore not claimed for packs with assets. Package extraction, signatures, and distribution trust are separate concerns.

Schema references are resolved only within the supplied schema document: local JSON Pointers, anchor references, and dynamic-anchor references are supported; unresolved references and external/relative resource loading are rejected. Legacy recursive-reference keywords fail closed under the Draft 2020-12 profile.

## Renderer compatibility audit

This audit keeps schema validity separate from renderer compatibility and target qualification.

Run:

    python tools/ambient_validation/check_renderer_compatibility.py contracts/examples/first-germination.scene.json
    python tools/ambient_validation/check_renderer_compatibility.py contracts/examples/first-germination.scene.json --json
    python -m unittest discover -s tools/ambient_validation -p 'test_*.py' -v

The compatibility CLI returns exit code 0 when the supplied profile declares support for every checked setting, 1 when the valid Scene Pack is incompatible, and 2 when input/schema/profile validation fails. Therefore the checked-in Sovereign Visual Core profile is expected to return exit code 1: it records the current blockers, not a supported renderer.

The profile pins the exact inspected Sovereign Boot source snapshot `ab6eac9641e0673cf2de36018a35986c122b36a8` and remains `source-inspection-only`. This snapshot adds `crates/visual-pack`, a duplicate-key-aware Scene Pack v1 parser that validates engine version `1.0.0` and maps the manifest to typed renderer settings. The browser-WASM adapter consumes the parser, and the WASI CLI and capability-denied Wasmtime host also use it. The browser/WASI rendering paths accept centered-network motion and the gradient-only static fallback; they reject non-empty safe regions, unsupported compositions, assets without a trusted resolver, required external capabilities, and inputs rather than silently dropping them. The exact-head upstream CI run was queued at the last check; parser presence is not build or runtime qualification. Keep the profile `source-inspection-only` until exact-head evidence completes.

The parser explicitly validates the fixture's `engineVersion` against `1.0.0`, and the capability profile now records that implementation plus the implemented strict manifest-to-settings mapping. That does not make the whole Scene Pack compatible: the current browser/WASI rendering adapters support only centered-network motion and gradient-only `staticFallback`, reject non-empty `safeRegions`, and do not implement the fixture's edge-biased/minimal compositions. Asset bytes still require a safe host resolver and actual SHA-256 verification; signed-pack/attribution policy is separate. Browser reduced-motion and document-visibility handling exist, but OS lock/suspend/wake behavior and whole-process memory accounting remain unqualified. The profile therefore continues to fail closed for those unsupported capabilities.

The audit also checks `accessibility.reducedMotionPresentation` as a selected variant, not just whether static-gradient pixels can be generated. A `staticFallback` primitive does not automatically implement a manifest preference for `idle` or `lockedBackground`; the chosen variant must be declared supported, or the compatibility report emits `accessibility.reduced_motion_variant.unsupported`.

A compatibility pass from a future profile still requires exact-head compile/runtime tests, effective-settings checks, golden RGBA output, resource enforcement, and host lifecycle evidence. This checker does not render images or qualify runtime behavior.


The compatibility checker distinguishes renderer-owned memory estimates from an operating-system-enforced whole-process memory ceiling, and separately checks scene-requested versus resource-budget branch ceilings. These contracts are not interchangeable.
