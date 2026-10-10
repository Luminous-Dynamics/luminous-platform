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

The profile is pinned to exact source commit c3b7bb9a376af9abd7bff7cdfc13472e2d44e82e in Luminous-Dynamics/sovereign-boot and is marked source-inspection-only. It is not a claim that the referenced commit builds or runs. Refresh the profile only after re-inspecting the exact new source head; never replace an unqualified state with a qualified label simply because a build starts or CI is queued.

A compatibility pass from a future profile still requires exact-head compile/runtime tests, effective-settings checks, golden RGBA output, resource enforcement, and host lifecycle evidence. This checker does not render images or qualify runtime behavior.
