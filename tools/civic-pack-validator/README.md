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
  --business-type sit_down_restaurant \
  --facts /tmp/openstart-facts.json
```

Example facts file (only use answers the person actually provided; omit unknown fields):

```json
{
  "handles_food": true,
  "business_entity_registration_intended": true
}
```

The evaluator uses a three-valued rule for every declared predicate:
- `true`: the predicate matches the submitted, correctly typed fact;
- `false`: the submitted, correctly typed fact contradicts the predicate, so that requirement is omitted as not applicable;
- `unknown`: the fact was not supplied, so the requirement remains visible as needing scope confirmation. Explicit malformed input (unknown fact key, wrong type, or non-object facts document) is rejected as `invalid_context` with no requirement cards.

Predicates use only a schema-allow-listed field and one of four operators (`eq`, `in`, `is_true`, `is_false`). Packs cannot submit code, expressions, regular expressions, imports, or dynamic function names. The evaluator does not parse explanatory free text as a rule. Jurisdiction and business type are locked to the explicit CLI selection and cannot be overridden through the facts file. Unknown applicability must not be interpreted as false or true.




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

## Source review receipts and history

Read the [source review and receipt protocol v0.1](../../docs/civic-services/source-review-protocol-v0.1.md). A source cannot claim a non-unreviewed state without a latest receipt matching its URL, snapshot digest, reviewer metadata, and reuse-rights fields. A `current` receipt requires all five review-scope fields to be true.

The fixture intentionally keeps both `source_review_receipts: []` and `requirement_review_receipts: []`, with null review pointers; no source or claim has been reviewed. Verified requirements need a receipt that hashes the exact claim-bearing fields and names all current source receipts supporting that claim. Pull-request CI checks each pack-changing commit against its first parent and rejects editing, removing, or reordering receipts added in any earlier PR commit. This is append-only history relative to the base; it does not authenticate reviewer identity or prove legal accuracy. Require independent human review and approved repository rules before publication.

For a local baseline comparison:

```sh
python tools/civic-pack-validator/check_review_history.py \\
  --baseline-pack /path/to/base-pack.json \\
  --pack examples/jurisdiction-packs/za-city-of-johannesburg-food-service-0.1.0.json
```

## Reproducible source snapshot manifests

The [source capture manifest contract](../../docs/civic-services/source-capture-manifest-v0.1.md) adds an offline helper that hashes bytes you have already captured; it does **not** fetch URLs, mark sources current, or create review receipts. It flags byte changes, redirect-target changes, and content-type changes against a prior manifest. Any change signal requires a human to review the source before the source/claim ledgers are updated.

Example:

```sh
python tools/civic-pack-validator/capture_source_snapshot.py \
  --source-id joburg.business-licences \
  --canonical-url https://joburg.org.za/work_/InvestInJoburg/Pages/Business-Licences.aspx \
  --snapshot /secure/capture/business-licences.html \
  --captured-at 2026-10-14T18:00:00Z \
  --final-url https://joburg.org.za/work_/InvestInJoburg/Pages/Business-Licences.aspx \
  --content-type text/html \
  --previous-manifest /secure/evidence/business-licences.manifest.json \
  --output /secure/evidence/business-licences.next-manifest.json
```

Do not put protected or third-party source bytes in the repository by default. Store them only where lawful and access-controlled. A manifest is metadata about the bytes, not the evidence itself and not a legal review. The timestamp and observed headers must come from the capture operator; this tool does not pretend to verify them independently.

## Applicability predicate contract

Read the normative [Applicability predicate contract v0.1](../../docs/civic-services/applicability-predicate-contract-v0.1.md) for the allow-listed fields, strict typing, three-valued conjunction semantics, and regression requirements.

Each requirement now contains at least one `applicability.predicates` record in schema version 0.1.0. Predicate evaluation is restricted to explicit facts; it is not a general rules language. Unknown answers lead to scope clarification. This limits the first version but provides a small, auditable core that can be expanded deliberately with tests rather than executing free-text conditions.

The schema constrains predicate fields and operators, while the semantic validator ensures predicate IDs are unique across the pack. A structurally valid predicate still does not prove the source-backed requirement is legally applicable: pack reviewers and the actual user's facts remain separate authorities.

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
