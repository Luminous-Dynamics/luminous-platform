# Luminous Ambient — Unified Visual Experience RFC 001

**Status:** Proposed architecture; not an implementation or qualification claim  
**Date:** 2026-10-10  
**Proposed product repository:** `Luminous-Dynamics/luminous-ambient`  
**Platform integration owner:** `luminous-platform`  
**Early boot owner:** `Luminous-Dynamics/sovereign-boot`

## Executive decision

Build one Luminous visual identity and one reusable scene/simulation model across boot, desktop wallpaper, idle/ambient display, login presentation, lockscreen background, suspend and resume. Do **not** make one process or one window responsible for every surface. Each surface has a different lifetime, permission boundary, rendering API and failure mode.

The recommended product is a dedicated public repository, provisionally named **luminous-ambient**. The current public repository inventory does not show a separate desktop ambient product. Do not assume the existing `symthaea-spore` name or the unfinished `spore-kernel` extraction means that product already exists. Before creating the repository, perform a final path/branch/asset ownership check; then create it only if no active source-of-truth exists. This RFC lives in the public platform repository so the integration boundary is reviewable now.

### Non-negotiable contracts

1. Boot presentation is optional. A missing binary, failed renderer, absent GPU, corrupt scene package or unavailable visual service must not prevent boot or recovery.
2. The ambient product never owns authentication, credential collection, lock authority, session privilege or NixOS mutation.
3. A wallpaper/layer-shell surface is not treated as a portable lockscreen. Lock-screen and greeter integrations must use the target desktop's supported, native mechanism.
4. One scene definition and design system can be shared; each presentation surface uses an explicit adapter and capability declaration.
5. Static/reduced-motion fallbacks are first-class and must render without a long-running GPU process.
6. Claims must be tested against an exact compositor, desktop, GPU, driver and package version. “Wayland supported” is not a sufficiently precise compatibility statement.

## 1. What exists today

### Public boot renderer

The public `sovereign-boot` source contains a Rust, CPU-oriented mycelial renderer:

- `crates/quicken-fb/src/mycelium.rs` defines a bounded branching network, seeded initial geometry, growth, pulses, contraction, and pixel-buffer rendering using Bresenham-style lines.
- `crates/quicken-fb/src/color.rs` defines an earthy solarpunk palette: deep moss, leaf green, lichen grey, clay, solar gold and white.
- The renderer is dependency-light at the rendering layer; the crate uses DRM/KMS for direct display access.
- The animation already has a useful identity: emergence, branching, node pulses and convergence. Its initial branch layout is deterministically seeded from a phrase.

Source references:
- https://github.com/Luminous-Dynamics/sovereign-boot/blob/main/crates/quicken-fb/src/mycelium.rs
- https://github.com/Luminous-Dynamics/sovereign-boot/blob/main/crates/quicken-fb/src/color.rs

### Important qualification blocker

The public standalone extraction has an open issue documenting missing monorepo-relative dependencies and binaries advertised by the extracted repository: https://github.com/Luminous-Dynamics/sovereign-boot/issues/3. Its renderer-hardening PR also explicitly scopes qualification to the standalone renderer rather than claiming that the unfinished lifecycle/recovery layer has been independently extracted: https://github.com/Luminous-Dynamics/sovereign-boot/pull/1.

**Do not couple ambient-runtime development to the missing state/recovery binaries, and do not treat the README's wider product language as implementation evidence.** Resolve the extraction discrepancy and qualify the exact standalone head on its own track.

### Visual assessment boundary

This assessment is based on source inspection, not on reviewed screenshots or a running machine. The current kernel is a meaningful procedural prototype, not yet a complete ambient-desktop system. The source shows no general scene-package format, compositor adapter, multi-display policy, user controls, native lock integration, or reduced-motion contract. The current CPU rasterizer is useful for a minimal early-boot mode; it should not dictate the fidelity or API of a future desktop renderer.

## 2. Visual direction

Create a recognizable living world rather than a sequence of unrelated splash screens.

### Signature scene: First Germination

Use the existing mycelial idea as the visual seed:

- Fine branching structures, nodes and light moving through a coherent network.
- A dark, low-noise substrate with restrained botanical green, teal and warm solar-gold accents.
- A strong hierarchy: one focal structure, negative space for desktop icons and clocks, no persistent status clutter.
- Gentle movement at rest; purposeful growth only when a meaningful, validated event calls for it.
- No compulsory audio, telemetry, cloud calls, camera, microphone or cognitive service.
- No text embedded into the wallpaper; text remains accessible, localizable UI owned by the host surface.

The boot sequence may grow and converge. The desktop state should settle into slower periodic motion. Idle/screen-dimming should simplify and reduce luminance. The lockscreen should favor legibility and stillness. Wake should restore a valid scene state without implying that the old process or GPU context survived the transition.

### Make it better than a novelty wallpaper

- **Compositional quality:** design for desktop icons, panels, clocks, ultrawide displays, portrait monitors and multi-monitor span/independent modes.
- **Temporal quality:** fixed-step simulation and explicit scene state; avoid animation that is only smooth when a process never pauses.
- **Rendering quality:** antialiasing, layered light and restrained glow should be tested against a CPU/static fallback. Do not port a heavy game renderer into early boot.
- **User control:** presets, motion intensity, frame-rate cap, static mode, brightness/contrast, monitor mapping and “pause while fullscreen/game is active.”
- **Accessibility:** reduced motion, contrast-safe variants, no essential meaning encoded only in color, and space reserved for native text.
- **Resource transparency:** report renderer health and estimated frame rate; define CPU/GPU/memory budgets and idle behavior before shipping.
- **Privacy:** reactive inputs are opt-in and local. Screen content, keystrokes, window titles and ambient sensors are out of scope by default.

## 3. Proposed component boundaries

### A. World/scene contract

Create a small, versioned declarative Scene Pack format. It describes assets, palette tokens, scene parameters, timing model, presentation variants, supported capabilities and fallback assets. It must not embed arbitrary executable plugins or privileged commands.

A scene package should declare at least:

- `schema_version`, stable `scene_id`, scene version and license/provenance for each asset.
- Seed policy and simulation version.
- Palette and material tokens, scene bounds, focal regions and safe text/icon regions.
- Desktop, boot, idle, locked-background and static variants (each variant is optional and capability-gated).
- Frame-rate ceiling, motion profile, reduced-motion variant and resource budget.
- Optional inputs and their privacy/consent requirements.
- A deterministic test fixture: initial state, fixed-step sequence, expected scene-state digest or golden captures.

Unknown critical fields, incompatible major schema versions and missing required assets fail closed for the scene load, but fail open for the machine: render the static default or host wallpaper. Never let package failure affect boot or lock authority.

### B. Scene simulation (Rust)

Keep the simulation independent of any windowing toolkit, desktop shell, GPU device or NixOS mutation API. Use deterministic seed handling and fixed-step progression; external events must be typed, validated and optional.

A simulation event may influence a visual transition; it must not be used as proof that an operating-system operation succeeded. Display only the state justified by trusted event evidence, and label approximate progress as approximate.

### C. Renderer

Benchmark two viable approaches before committing:

1. **In-process scene renderer:** suitable where the desktop adapter can host a Rust-generated GPU surface or texture without turning the desktop integration into a second renderer.
2. **Shared renderer/display protocol:** a Rust daemon renders one scene and publishes frames/state to adapters. DMA-BUF and synchronization fences can avoid unnecessary CPU copies on supported Linux stacks; complexity, driver behavior and lifecycle recovery must be measured.

A likely desktop candidate is `wgpu`, but that is a hypothesis until tested on target hardware. Keep the boot renderer separate and CPU-capable. Do not introduce GPU or desktop libraries into its required dependency graph.

### D. Presentation adapters

The adapter presents a scene and forwards only explicit capability/lifecycle signals. The core must not link against KDE, COSMIC, GNOME, SDDM or a specific compositor.

Adapters should report capability facts (for example: wallpaper surface, per-display pause, fullscreen/game detection, native lock-background support), not claim universal support based only on a desktop name.

### E. Settings and packaging

A user-facing settings app manages scenes, display assignments and resource controls. It does not own the renderer's authority or the lock state. Package the app, daemon, adapters, themes and NixOS modules separately so a user can install only the parts their desktop supports.

## 4. Desktop and security matrix

| Surface / desktop | Approach | Boundary / caveat |
|---|---|---|
| Sovereign Boot | Existing DRM/KMS renderer | Early boot, optional, CPU fallback; do not import desktop stack |
| KDE Plasma 6 wallpaper | Native Plasma/Qt Quick wallpaper plugin, with shared renderer integration if measured useful | Plasma's wallpaper plugin is a native QML extension; package against tested Plasma/Qt versions |
| COSMIC wallpaper | First evaluate upstream shared display/layer-shell integration; add a native adapter only where required | Test current COSMIC release/protocols. Pause/window-state behavior is version-specific |
| wlroots-family compositors | Optional `zwlr_layer_shell_v1` background client | Only compositors exposing the protocol; not a universal Wayland feature |
| GNOME | Native shell/display adapter or a proven shared display integration | Extension/Shell compatibility must be versioned |
| Login greeter (e.g. SDDM) | Theme/package that consumes static or supported animated assets | Greeter lifecycle and authentication remain independent |
| Session lockscreen | Host-native lockscreen background integration | Never overlay an ordinary wallpaper surface and call it a secure lockscreen |
| Idle/screensaver | Host idle policy plus renderer pause/reduced-motion state | Distinguish display blanking, idle animation and session locking |
| Suspend/resume | Lifecycle-aware pause and state reconstruction | Do not assume process, IPC or GPU context survives suspend |

For COSMIC in particular, recent upstream work has made background/layer-shell display on the lock screen possible through a COSMIC-specific protocol. This is evidence that integration is feasible, not evidence that a generic client is automatically safe or supported on every version. The implementation must use the compositor's documented path and preserve its security policy.

## 5. Upstream research and reuse decision

**Reuse interfaces and lessons before building a second Linux wallpaper stack.**

1. **Waywallen:** https://github.com/waywallen/waywallen  
   Its documented desktop matrix includes Plasma and several layer-shell integrations, including COSMIC. Its ecosystem already has video/image handling and optional Wallpaper Engine compatibility. Evaluate it as an integration reference or optional backend before reimplementing library management, playback, display selection and pause behavior.

2. **Waywallen Display:** https://github.com/waywallen/waywallen-display  
   Its public design includes a versioned display protocol, DMA-BUF frames and acquire/release synchronization fences, a Qt 6 QML item, a GObject bridge and separate Plasma/GNOME/layer-shell adapters. This is a concrete reference for separating rendering from presentation. Review protocol compatibility, lifecycle behavior, license, build dependencies and trust boundary before adopting any code.

3. **Upstream release evidence:** https://github.com/waywallen/waywallen-display/releases  
   Release v0.4.0 (2026-09-21) records daemon-controlled wallpaper transitions and COSMIC window-state tracking/automatic pause. The moving README and feature matrix have shown differences over time; pin an exact release/commit and test the required capability rather than copying a moving feature claim.

4. **KDE developer docs:** https://develop.kde.org/docs/plasma/wallpapers/  
   Plasma exposes a native QML wallpaper plugin path. Use that rather than pretending a generic background window gives first-class Plasma integration.

5. **COSMIC upstream:** https://github.com/pop-os/cosmic-epoch/releases  
   Recent release notes describe a COSMIC background component and a compositor change allowing layer-shell surfaces such as the background/on-screen keyboard on the lock screen. Treat this as COSMIC-specific integration and test login, lock and unlock behavior explicitly.

6. **Wallpaper Engine compatibility:** treat the third-party `open-wallpaper-engine` path as an optional compatibility feature, not as the scene format or runtime foundation. External renderer plugins are executable code; do not auto-install or silently trust user-downloaded plugins.

### Reuse policy

Start with a short, pinned upstream spike: build the existing display stack on the chosen NixOS target, capture its process/GPU/CPU behavior, test pause/resume across desktop/lock/suspend, and document integration gaps. If it gives us stable display and lifecycle plumbing, write our scene engine and settings on top of that boundary. If it does not, record the specific failing requirement before writing our own adapter.

Do not vendor or fork a large project simply because it has a feature matrix we like.

## 6. Repository topology

### Recommended public repo: `luminous-ambient`

Create this as a separate repository only after verifying no active equivalent exists in current public branches/assets. It should own:

- Rust scene/simulation core and versioned Scene Pack schema.
- Desktop renderer, renderer service/protocol if justified by benchmark.
- Plasma/COSMIC/GNOME/layer-shell adapters and the capability matrix.
- Settings UI, scene library/manifest, static assets and visual tokens.
- Nix packages/modules for the ambient product.
- Capture fixtures, compositor VM tests, performance baselines and supported-version policy.

### Keep other repositories narrow

- **`sovereign-boot`:** DRM/KMS renderer and boot-only dependencies, safety/failure semantics and early-boot tests. Export only a documented visual contract or static scene artifact once the standalone build/evidence boundary is resolved.
- **`luminous-platform`:** integration RFC, compatibility matrix, cross-repo composition and release/qualification policy. It must not duplicate renderer source.
- **`luminous-edge`:** add ambient packaging only after the product is independently buildable and opt-in by default. Do not make the ambient product a boot dependency.
- **Shared upstream visual source:** inventory before copying. Every transfer needs source path/revision, destination path/revision, license, digest, test evidence and explicit disposition. Retain/deprecate/archive rather than delete until parity is demonstrated.

Do not split out an assets-only repository initially. Keep licensed scenes with the runtime; extract assets only if independent releases or substantial downstream consumption prove that boundary is useful.

## 7. Delivery plan and acceptance evidence

### M0 — Inventory and upstream spike

- Record exact HEAD/branches, owners, and existing assets for the public boot, platform and meta-flake repositories.
- Reconcile the state of sovereign-boot issue #3 and PR #1; no new dependency on unfinished extraction code.
- Pin a Waywallen/Waywallen Display release and record Nix build results, source licenses and known limitations.
- Create a source-to-destination migration matrix. Do not move files yet.

**Exit:** immutable inventory and reproducible evidence packet; all unverified items remain explicitly unverified.

### M1 — World contract and first scene

- Define Scene Pack v1, deterministic fixed-step simulation contract and static fallback.
- Build one signature scene: First Germination.
- Render a fixed set of canonical captures at 1920×1080, 2560×1440, 3440×1440 and 1080×1920 (unsupported resolutions are recorded rather than silently skipped).
- Review captures for line quality, focal balance, icon/clock safe areas, reduced-motion and low-contrast modes.

**Exit:** schema fixtures, stable scene-state outputs, golden capture metadata and no host-level dependencies.

### M2 — Desktop proof

- Prove one Plasma 6 wallpaper path and one COSMIC path using the same scene contract.
- Test two displays with mixed resolutions, compositor restart, desktop shell restart, fullscreen pause, user logout/login and GPU device loss.
- Capture CPU, GPU, memory and frame-time baselines on the same reference machine.

**Exit:** both adapters pass their declared capabilities; unsupported lock/idle functions are explicit rather than simulated as success.

### M3 — Lock, greeter and lifecycle

- Add a static/reduced-motion theme to a supported greeter and the native session-lock integration for each target desktop.
- Test that visual processes never accept credentials, never weaken the lock state, and cannot hold up boot, login recovery or unlock.
- Exercise suspend/resume, monitor hotplug and renderer crash. Missing renderer must leave the host surface and authentication mechanism functional.

**Exit:** negative security tests and lifecycle evidence captured per desktop/version.

### M4 — Package and maintain

- Add opt-in NixOS packaging and desktop packages; lock inputs.
- Publish a capability/version matrix and exact build/test commands.
- Keep CI, local simulation, VM, visual capture and hardware tests distinct. A queued/skipped test is not a pass; a build is not a visual or lifecycle pass.

## 8. Initial measurable budgets (provisional targets, to be benchmarked)

These are engineering goals, not measured results:

- Static fallback available for every scene.
- Ambient default capped at 30 FPS; allow 60 FPS as an explicit higher-quality option.
- Pause rendering when the output is asleep and when the host surface is not visible, where the compositor exposes reliable state.
- Target ≤2% average CPU on the reference desktop at idle and 0% ongoing renderer CPU while paused; report actual hardware/driver and measurement method. Do not claim this target until measured.
- Keep the boot renderer's existing small dependency graph separate from desktop dependencies.
- No network, microphone, camera or window-content access in the default scene runtime.
- Every claimed supported capability has an automated or documented manual test on the exact target version.

## 9. Open decisions requiring evidence (not taste alone)

1. Can Waywallen Display be used as a stable presentation backend without forcing its library/plugin model on our scene engine?
2. What is the best Rust/GPU renderer path on the reference NixOS target, including Mesa and NVIDIA where applicable?
3. Which COSMIC release and protocol provide the required background, pause and lock-background behavior?
4. Which desktop lock/greeter surfaces expose a supported background-only integration that preserves secure input and transitions?
5. Does the existing CPU scene prototype produce good enough geometry once antialiasing, lighting, compositional safe zones and alternate aspect ratios are considered?
6. Which existing scene/assets can be legally and reproducibly transferred without depending on an unavailable monorepo path?

## Definition of done

One scene package renders a visually coherent sequence across boot, desktop, idle and supported lock/greeter surfaces; per-surface adapters announce their actual capabilities; the renderer adapts or pauses safely; unsupported paths degrade to a static image; secure authentication remains owned by the desktop; Nix packages are reproducible; and exact-head visual, resource and lifecycle evidence exists for every advertised target.

This RFC authorizes none of those completion claims by itself.


## 10. Contract artifacts and existing visual work discovered during follow-up

A first, intentionally data-only Scene Pack v1 contract is now included in this branch:

- [JSON Schema](../../contracts/ambient-scene-pack-v1.schema.json)
- [First Germination fixture](../../contracts/examples/first-germination.scene.json)
- [Scene Pack semantics and loader validation contract](AMBIENT_SCENE_PACK_V1.md)

The fixture is procedural-only (`assets: []`) so it does not accidentally pull artwork of unclear origin into the product. It includes explicit boot, desktop, idle, locked-background and static-fallback variants, safe regions for desktop UI, a fixed simulation seed, resource ceilings, and opt-in-only inputs. The schema rejects unknown fields. The accompanying semantic contract adds checks JSON Schema cannot express by itself, including canonicalized asset path containment, checksum verification, safe-region bounds and cross-field fallback/resource invariants.

These files are contract artifacts, not evidence that a scene renderer, importer, UI or compositor adapter already exists. The first validation during authoring checked schema shape, required fields, types, enum/constant values, ranges, patterns, unknown fields, presentation FPS limits, safe-region bounds and the static-fallback semantics. That was a purpose-built structural validation, not yet a standards-complete JSON Schema validator or runtime conformance suite. A real consumer implementation must add independent schema validation and negative fixtures before this contract is called qualified.

### Exact upstream review targets

The current upstream candidates were pinned for further evaluation (not adopted as production dependencies):

- `waywallen` v0.4.4: commit `f42cb1a6b12301dfd1b46e1ef1cfbce6140bc5d3`.
- `waywallen-display` v0.4.0: commit `4fc25d632125967de0be34d095206ca9420432e8`.

The upstream GitHub release notes document daemon-controlled transitions, Plasma/GNOME/layer-shell integrations, COSMIC window-state pause, and fixes around successful presentation and renderer respawn. I fetched each `LICENSE` at the exact pinned commit; both are MIT (copyright `hypengw`, 2026). This confirms the headline license only—not transitive dependency provenance, patent posture, or whether code transfer is the right choice. The candidate architecture can reuse the display protocol and adapters without inheriting Waywallen's entire wallpaper library or input policy. In particular, do not enable app-title-based exclusion rules or other window-content inputs by default.

- [Waywallen v0.4.4 release](https://github.com/waywallen/waywallen/releases/tag/v0.4.4)
- [Waywallen Display v0.4.0 release](https://github.com/waywallen/waywallen-display/releases/tag/v0.4.0)

### Existing Mycelix visual foundation

An internal Mycelix visual-foundation change is in progress. It introduces semantic palette tokens, quieter effects, local system-font fallbacks, focus-visible and reduced-motion rules for Mycelix Pulse and Sensorium. Its review description still keeps browser builds, deterministic screenshots and full accessibility qualification pending.

Use this design-system work as an input only after source ownership and review: the dark canvas (`#0A100E`) and mineral teal (`#76D9C1`) can map to semantic ambient tokens, while the existing boot renderer retains its early-boot palette and dependency boundary. Do not copy app CSS or make an unqualified UI change a required build dependency for the boot renderer. Preserve the rule: shared visual language, domain-owned interactions.
