# Ambient Scene Pack v1

**Status:** contract proposal; renderer implementation and runtime qualification are separate work  
**Schema:** [ambient-scene-pack-v1.schema.json](../../contracts/ambient-scene-pack-v1.schema.json)  
**Example:** [first-germination.scene.json](../../contracts/examples/first-germination.scene.json)  
**Visual concept (not a runtime asset):** [First Germination animated SVG](concepts/first-germination-concept.svg)

## Purpose

A Scene Pack describes a Luminous visual world and its surface-specific variants. It is data, not an executable plugin. The same scene identity can be presented by a boot renderer, desktop wallpaper adapter, idle state or a native lockscreen background integration without implying that those surfaces share the same security or lifecycle behavior.

## v1 guarantees

- JSON Schema Draft 2020-12 validation with unknown fields rejected.
- Deterministic simulation identity: `sceneId`, `sceneVersion`, engine ID/version, seed, and fixed-step frequency are explicit.
- Surface behavior is distinct: boot, desktop, idle, locked background, static fallback.
- Every conforming pack declares a static fallback and reduced-motion behavior. A `gradient-only` fallback is a deterministic linear gradient from `palette.canvas` at the upper-left to `palette.substrate` at the lower-right; it needs no external asset or animation.
- The manifest carries an explicit resource ceiling and pause expectations.
- Optional inputs are limited to local time, pointer position, and audio level. The pack cannot turn them on by default; user consent is required at runtime. `audio-level` means locally available system playback/output level only—not microphone capture.
- Asset paths are relative package paths and each file is pinned by SHA-256 and an SPDX license identifier.
- No field allows arbitrary shell commands, shared-library paths, dynamic executable code, credentials, or system mutation.
- The locked-background variant is purely visual. It does not authenticate, lock, unlock, or receive credentials.

## Validation pipeline

A loader should process a pack in this order:

1. Parse JSON while rejecting duplicate object keys and invalid UTF-8.
2. Validate the document against the exact v1 schema; reject unknown fields.
3. Apply semantic checks that JSON Schema alone cannot express:
   - unique `assetId` and `input.id` values;
   - every declared asset path remains inside the unpacked package after canonicalization, with no symlink escape;
   - each asset's SHA-256 matches the exact file bytes;
   - every asset license and origin is preserved in the package's attribution manifest;
   - each safe region satisfies `x + width <= 1` and `y + height <= 1`;
   - the static fallback uses `motion = none`, `maxFps = 0`, and `composition = gradient-only`;
   - any presentation's `maxFps` does not exceed `resourceBudget.maxFps`;
   - every required renderer capability is actually supported by the selected adapter.
4. Enforce the manifest's branch, memory and frame-rate limits in the renderer; never trust UI-only limits.
5. Select an exact supported presentation. If unsupported, malformed or over budget, use the packaged static fallback or host wallpaper.
6. Record the pack digest, selected variant, renderer version, adapter identity and fallback reason in a local diagnostic receipt. Do not include user activity or secret inputs.

A schema-valid pack is only a valid declaration. It is not proof of performance, visual quality, compositor compatibility, or lockscreen safety.

## Stable identity and hashing

- Preserve exact source bytes and their SHA-256 digest for each asset.
- For a future signed distribution profile, canonicalize the manifest using RFC 8785 JSON Canonicalization Scheme and use a domain-separated digest: `SHA-256(UTF8("luminous-ambient-scene-pack-v1\\0") || JCS(manifest))`.
- Do not treat a digest alone as authorization or publisher identity. Distribution signatures and trust policy are a later, separate contract.
- A fixed seed only guarantees repeatability when the exact simulation engine/version, numeric rules and fixed-step sequence are the same. Cross-GPU pixel identity is not assumed; deterministic simulation state and renderer-specific golden captures are separate assertions.

## Presentation semantics

| Variant | Intent | Initial default |
|---|---|---|
| `boot` | Growth and convergence, driven only by independently validated boot observations | Up to 30 FPS |
| `desktop` | Slow, low-contrast motion that leaves space for icons and panels | Up to 30 FPS |
| `idle` | Reduced motion and luminance | Up to 10 FPS |
| `lockedBackground` | Minimal background motion; native lock UI owns text and authentication | Up to 5 FPS where supported |
| `staticFallback` | No animation or GPU dependency | 0 FPS |

These are candidate limits, not measured resource claims. Adapters can apply lower limits based on current power policy, accessibility settings, hardware and compositor state.

## Boot and lock boundaries

- Boot observations can influence a transition only through a typed and validated input channel. A scene state must never be used as evidence that a service, filesystem, recovery step or boot transaction succeeded.
- The wallpaper process must never need to execute with elevated privileges.
- A missing renderer, bad asset, unsupported format or malformed manifest cannot delay boot, prevent recovery, block unlock, or alter the greeter's authentication UI.
- Native desktop locking remains authoritative. A wallpaper or layer-shell process cannot substitute for it.
- `onWake = reinitialize-from-seed` describes visual recovery, not continuity of the old process or GPU context.

## First Germination fixture

The included fixture intentionally uses no external assets (`assets: []`) so the first renderer can implement its procedural network without inheriting unreviewed media. Its palette maps the existing boot colors to semantic roles and uses the Mycelix visual foundation's dark canvas and mineral-teal accent as a cross-product bridge. This is a design proposal—not yet a pixel-tested theme, runtime implementation or merged dependency on the Mycelix visual PR.


## Visual concept reference

The linked SVG is a self-contained 16:9 concept scene with slow signal traces, subtle node pulses, sparse particle drift, and a `prefers-reduced-motion` rule. It is stored under architecture concepts, not in the sample pack's asset list. It establishes composition and motion intent for review; it is not generated by the Rust scene engine, and it does not prove performance, color management, per-display scaling, or compositor behavior.
