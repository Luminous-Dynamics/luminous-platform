# Civic jurisdiction-pack validator

This tool validates versioned jurisdiction packs and emits a deterministic JSON report. It does not determine what the law is, verify that a source's content is legally correct, or substitute for human source review.

## Run locally

Run the deterministic workflow evaluator separately to verify exact jurisdiction/workflow/business-type matching. On the intentionally unpublished Johannesburg fixture it must return `pack_not_published` with no executable requirement cards:

```sh
python tools/civic-pack-validator/evaluate_workflow.py \
  --schema schemas/jurisdiction-pack-v0.1.0.schema.json \
  --pack examples/jurisdiction-packs/za-city-of-johannesburg-food-service-0.1.0.json \
  --as-of 2026-10-13T15:00:00Z \
  --jurisdiction za/gauteng/city-of-johannesburg \
  --workflow business.restaurant-launch \
  --business-type sit_down_restaurant
```



From the repository root:

```sh
python -m venv .venv
. .venv/bin/activate
python -m pip install -r tools/civic-pack-validator/requirements.txt
python tools/civic-pack-validator/validate_pack.py \
  --schema schemas/jurisdiction-pack-v0.1.0.schema.json \
  --pack examples/jurisdiction-packs/za-city-of-johannesburg-food-service-0.1.0.json \
  --as-of 2026-10-13T15:00:00Z \
  --report /tmp/jurisdiction-pack-report.json
PYTHONPATH=tools/civic-pack-validator python -m unittest discover -s tools/civic-pack-validator/tests -v
```

Use a frozen, timezone-aware `--as-of` instant when generating evidence that must be reproducible. Different timestamps may legitimately change freshness outcomes. The report includes hashes of the exact pack and schema bytes so a reviewer can identify the validated inputs.

## Exit codes

- `0`: schema/semantic validation passed. Publication may still be blocked.
- `1`: malformed JSON/schema or one or more integrity/safety checks failed.
- `2`: validation passed but `--require-publishable` was requested and publication gates are blocked.

A draft pack can therefore be a structurally valid fixture while being explicitly **not publishable**. The Johannesburg example intentionally behaves this way: its source records are unreviewed, requirements remain draft, and no legal-completion claim can be made from it.

## Dependency reproducibility

The runtime packages are pinned to exact versions in `requirements.txt`, including the `jsonschema` transitive dependency graph. This constrains version resolution but does **not** provide pip artifact-hash locking or a fully offline supply chain. Before production use, generate and review a platform-specific hash-locked requirements file (or equivalent lockfile) and retain provenance for the downloaded wheels.

## What is checked

- JSON Schema Draft 2020-12, including URI/date-time format checks;
- stable uniqueness of source, requirement, and action identifiers;
- resolution of every requirement `source_refs` value;
- requirement business types constrained to the workflow's declared scope;
- required decision-contract safety constants;
- source-review state, reviewer metadata, and review deadline against the frozen timestamp;
- publication gate outcomes and a reproducible JSON report with schema and input SHA-256.

JSON Schema and this semantic validator cannot prove legal accuracy, detect paraphrases of third-party source text reliably, authenticate government systems, or establish whether a source is authoritative. Human reviewers must confirm applicability and content before publication. Change monitoring should alert for review; it must never silently re-approve a changed source.
