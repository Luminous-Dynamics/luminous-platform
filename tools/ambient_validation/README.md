# Ambient Scene Pack validator

The CLI validates the schema as JSON Schema Draft 2020-12, applies format checks, then checks cross-field invariants and packaged asset files. It rejects duplicate object keys, invalid UTF-8, non-standard numeric constants, and external schema references.

Run locally (from the repository root):

    python -m pip install -r tools/ambient_validation/requirements.txt
    python tools/ambient_validation/validate_scene_pack.py contracts/examples/first-germination.scene.json
    python tools/ambient_validation/test_validate_scene_pack.py

For asset-bearing packs, pass --asset-root with the unpacked package root. Renderer capabilities must be explicitly declared with repeated --supports-capability options; unsupported requirements fail closed.

This validator is not a filesystem sandbox against concurrent mutation and does not qualify rendering, performance, desktop lifecycle, or lockscreen security. It verifies each asset's SHA-256 and per-asset license identifier, but Scene Pack v1 still lacks a separate attribution-manifest schema; full attribution-manifest completeness is therefore not claimed for packs with assets. Package extraction, signatures, and distribution trust are separate concerns.
