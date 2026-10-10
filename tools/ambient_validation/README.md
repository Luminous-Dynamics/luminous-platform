# Ambient renderer compatibility audit

This audit keeps Scene Pack schema validity separate from renderer compatibility and target qualification.

Run from the repository root:

    python tools/ambient_validation/check_renderer_compatibility.py contracts/examples/first-germination.scene.json
    python tools/ambient_validation/check_renderer_compatibility.py contracts/examples/first-germination.scene.json --json
    python -m unittest discover -s tools/ambient_validation -p 'test_*.py' -v

The CLI returns exit code 0 when the supplied profile declares support for every checked setting, 1 when the valid Scene Pack is incompatible, and 2 when input/schema/profile validation fails. Therefore the checked-in Sovereign Visual Core profile is expected to produce exit code 1: it records the current blockers, not a supported renderer.

The profile is pinned to exact source commit c3b7bb9a376af9abd7bff7cdfc13472e2d44e82e in Luminous-Dynamics/sovereign-boot and is marked source-inspection-only. It is not a claim that the referenced commit builds or runs. Refresh the profile only after re-inspecting the exact new source head; never replace an unqualified state with a qualified label simply because a build starts or CI is queued.

This checker does not render images and cannot qualify runtime behavior. Its purpose is to prevent unsupported fields from being silently treated as applied. A compatibility pass from a future profile still requires exact-head compile/runtime tests, effective-settings checks, golden RGBA output, resource enforcement, and host lifecycle evidence.
