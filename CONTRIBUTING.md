# Contributing to Luminous Platform

Thanks for helping make Luminous portable, accessible, and safe to adapt. This repository is the canonical home for shared contracts and architecture proposals; renderer implementations live in their own repositories and must qualify against those contracts.

## Start small

1. Pick the layer you want to improve: a declarative Scene Pack, schema/validator, renderer adapter, documentation, or test fixture.
2. Read the relevant contract before changing code. For ambient scenes, start with [Ambient Scene Pack v1](docs/architecture/AMBIENT_SCENE_PACK_V1.md) and the [Scene Pack JSON Schema](contracts/ambient-scene-pack-v1.schema.json).
3. Make the smallest change that can be validated independently. Add a positive test and, where relevant, a negative/adversarial test.
4. Run the applicable checks locally and report their actual outcomes in the pull request. An authored test, a queued run, source inspection, and a completed passing run are different kinds of evidence.

Please avoid bundling unrelated repository, schema, renderer, and product changes into one pull request. For a contract change, describe compatibility implications and identify every consumer that pins or implements it.

## Authoring a community scene/theme

A **theme** is a complete, reusable Scene Pack with a stable identity and a clear visual idea. A **presentation variant** describes how the same pack should behave on a particular surface or accessibility/power mode. These are different concepts: the current First Germination example is one pack with five variants (`boot`, `desktop`, `idle`, `lockedBackground`, and `staticFallback`), not five separate themes.

### 1. Start from the example, then give the pack its own identity

Copy `contracts/examples/first-germination.scene.json` to a new file in a draft fork or branch. Do not edit the shared example merely to try a palette.

- Choose a stable, globally distinctive `sceneId`; do not change it for ordinary artwork revisions.
- Use a valid SemVer `sceneVersion`. Increment the version when the scene's declared content or behavior changes; explain compatibility-sensitive changes.
- Keep `simulation.engine` and `engineVersion` aligned with an engine the target adapter actually supports.
- Give the pack a descriptive title, meaningful license metadata, and a short description that explains its identity.
- Keep presentation choices, simulation limits, and resource limits declarative. A manifest is data, not a script or plugin.

Useful theme directions might include organic/mycelial, orbital/celestial, auroral/fluid, oceanic/bioluminescent, geological/desert, fractal/botanical, and quiet geometric scenes. These are portfolio prompts, **not claims that those packs already exist**. Prefer seven genuinely distinct, well-finished launch themes to seven palette swaps; after that, grow an open catalogue rather than imposing a permanent theme-count limit.

### 2. Preserve accessibility and resource budgets

Every pack must retain a valid static fallback. Do not make color the only way to convey meaning. Specify motion honestly, respect the declared frame-rate and branch budgets, keep safe regions inside the normalized viewport, and do not assume that a host supports every presentation or capability.

A declared budget is a limit to be enforced, not proof that the real process stays within it. Renderer-owned memory estimates do not establish a whole-process or GPU memory ceiling. A theme manifest must never be able to authorize display access, networking, filesystem access, lock/unlock, boot control, or privileged execution.

### 3. Keep assets reviewable

Begin with `assets: []` when procedural rendering is sufficient. If assets are necessary:

- Include only assets that are licensed for the intended distribution and record the required license/attribution information.
- Use safe relative paths; never use absolute paths or parent traversal.
- Supply the expected SHA-256 digest and include the asset bytes in the reviewable pack submission.
- Do not make scene loading fetch remote assets or follow arbitrary URLs implicitly.
- Expect the selected host adapter to resolve paths safely (including symlinks) and verify bytes before use. A manifest's digest declaration does not itself prove that an asset was checked.

Do not include secrets, personal data, opaque executables, scripts, or hidden network dependencies in a theme pack.

### 4. Validate before submitting

From the repository root, create an isolated Python environment and install the repository's pinned validator dependencies:

```sh
python -m venv .venv
. .venv/bin/activate
python -m pip install --disable-pip-version-check -r tools/ambient_validation/requirements.txt
python tools/ambient_validation/validate_scene_pack.py contracts/examples/first-germination.scene.json
python -m unittest discover -s tools/ambient_validation -p 'test_*.py' -v
```

These commands validate the canonical example and run the current contract tests; they do **not** automatically validate an arbitrary new file unless you pass that file to the validator, nor do they qualify a renderer. Add your own pack path to the command while developing.

If you change the schema, validator, shared negative-fixture corpus, or pinned source snapshot, run all relevant checks. Update the contract tests and snapshot provenance deliberately; do not silently edit a vendored copy to make it diverge from the canonical source. A passing Python contract lane does not imply that Rust parsing, browser rendering, Wasmtime execution, compositor behavior, or physical boot has passed.

### 5. Test the experience, not just the JSON

For a visual pack contribution, include a review note covering:

- Intended visual identity and what makes it distinct from existing packs.
- Palette roles, composition, motion profile, reduced-motion/static behavior, and icon/text safe regions.
- Required and optional host capabilities, with unsupported requirements failing closed.
- Resource limits, asset/license provenance, and known adapter limitations.
- At least one deterministic preview or reproducible capture plan, with the exact renderer revision and commands used.
- What was actually tested, what was not tested, and the precise platform/surface scope.

Do not call a pack "cross-platform" solely because the schema parses. Each adapter must declare what it supports, and each target/surface needs its own qualification evidence.

## Extending the contract safely

The current schema intentionally rejects unknown fields. Do not loosen `additionalProperties` to make one theme's custom metadata work, and do not invent unregistered capability names.

When an extension is genuinely needed, propose it separately with:
- a namespace and owner;
- a versioned definition and strict schema;
- explicit required/optional semantics and safe behavior when unsupported;
- compatibility and migration rules;
- positive, negative, and adversarial fixtures;
- a statement of whether older consumers must reject, ignore, or safely degrade that extension.

Keep new creative content independent from the renderer's core where possible. A new palette or asset set should usually be a new pack, not a new code path. A new simulation engine or host capability needs an explicit contract and adapter review.

## Pull request checklist

- [ ] The change has one clear purpose and keeps the canonical contract authoritative.
- [ ] Scene identity, version, license and asset provenance are complete.
- [ ] Unknown fields, invalid paths, excessive budgets and unsupported capabilities fail closed.
- [ ] Accessibility, static fallback and host/surface limitations are documented.
- [ ] Relevant positive and negative tests were added or updated.
- [ ] Validation commands and their actual outcomes are recorded.
- [ ] The PR separates source-reviewed, test-confirmed, runtime-qualified and platform-qualified claims.

A contribution that is incomplete but accurately scoped is welcome. Please label a prototype as a prototype rather than inflating its qualification state.
