# Applicability predicate contract v0.1

**Status:** Proposed, executable contract  
**Schema:** `jurisdiction-pack-v0.1.0.schema.json`  
**Purpose:** Make simple applicability questions deterministic without turning guidance text into executable code.

## Non-goals

These predicates do not decide what the law requires, prove a source is authoritative, establish legal eligibility, or replace human review. They only evaluate a small set of explicit facts against a reviewed pack's declared predicate records.

An applicability result of `applicable` is not a permit, legal conclusion, authorization, or evidence that any government process has been completed.

## Predicate record

Every requirement declares one or more predicates:

```json
{
  "predicate_id": "licence.handles-food",
  "field": "handles_food",
  "operator": "is_true"
}
```

The JSON Schema constrains the field and operator set, scalar value shapes, and whether a value must be present. The semantic validator enforces predicate-ID uniqueness across the complete pack.

Supported fields:

| Field | Type | Supplied by |
|---|---|---|
| `jurisdiction_id` | string | Explicit evaluator selection; never overridden by user facts |
| `business_type` | string | Explicit evaluator selection; never overridden by user facts |
| `handles_food` | boolean | Explicit user answer or a separately validated trusted intake record |
| `prepares_food_on_site` | boolean | Explicit user answer or a separately validated trusted intake record |
| `sells_perishable_food` | boolean | Explicit user answer or a separately validated trusted intake record |
| `premises_status` | non-empty string | Explicit user answer; value vocabulary needs a future versioned enum |
| `sells_alcohol` | boolean | Explicit user answer or a separately validated trusted intake record |
| `offers_seating` | boolean | Explicit user answer or a separately validated trusted intake record |
| `business_entity_registration_intended` | boolean | Explicit user answer |

The current engine accepts no free-form extra fact keys. It validates the expected primitive type exactly; for example, integer `1` is not a boolean `true`. A malformed fact document, unknown key, or wrong type returns `invalid_context` and no requirement cards.

The first implementation must not extract predicate facts from arbitrary natural-language answers, source pages, or AI output. UI answers should be stored as explicit assertions with their origin and timestamp in a later service contract; this version consumes only the typed JSON context supplied to the evaluator.

## Operators

- `eq`: strict type-and-value equality with `value`.
- `in`: strict type-and-value membership in the non-empty `value` array.
- `is_true`: matches only the boolean `true`; boolean `false` contradicts it.
- `is_false`: matches only the boolean `false`; boolean `true` contradicts it.

No regular expressions, arithmetic, nested expressions, recursive predicates, dynamic function names, imports, or user-supplied code are supported. Version 0.1 deliberately uses a small allow-list.

## Three-valued logic

Each predicate evaluates to exactly one result:

- `true`: the typed fact is present and matches.
- `false`: the typed fact is present and contradicts the predicate.
- `unknown`: the referenced fact is absent (or is null/empty in an internal call).

Predicates within one requirement are combined as a logical AND:

1. If **any** predicate is false, the requirement is `not_applicable` for this selected context and it is not returned as a guidance card.
2. Otherwise, if **any** predicate is unknown, return the requirement with `needs_scope_confirmation`.
3. Only if **all** predicates are true may later review/evidence gates determine which guidance state is shown.

Unknown is not false. No implicit default values are applied. A caller that supplies malformed facts is rejected as `invalid_context`, rather than allowed to influence a requirement.

## Evaluation sequence

1. Validate the pack against the exact JSON Schema and semantic integrity rules.
2. Resolve the selected jurisdiction and workflow by exact identifier match.
3. Verify the business type is in the workflow's declared scope.
4. Reject unsupported fact fields or types; reserve jurisdiction and business type for evaluator-controlled values.
5. Evaluate allow-listed predicates and expose only their IDs, fields, operators, and three-valued outcomes—not a copy of the entire input fact record.
6. Exclude false predicates; require clarification for unknowns.
7. Apply pack publication and source-review gates before current guidance can be emitted.
8. Keep user progress and government outcomes outside predicate evaluation. An official outcome remains unknown until it is independently evidenced.

## Required regression cases

- Missing boolean fact yields `unknown` and `needs_scope_confirmation`.
- True boolean fact satisfies `is_true`; false boolean fact excludes that requirement.
- Wrong type (including integer `1` where boolean is required) returns `invalid_context`.
- Unknown keys and non-object fact documents return `invalid_context` with no cards.
- Wrong selected jurisdiction/workflow/business type returns scope confirmation or out-of-scope, never a neighboring pack's rules.
- Unpublished pack returns no requirement cards even when all facts match.
- A structurally valid pack with unreviewed or stale evidence returns `publication_blocked`, not `invalid_pack`.
- No predicate result can create an authority outcome, approval, or legal-compliance state.

## Extension policy

Any expansion of fields, operators, enum vocabularies, or combination semantics requires:
- a schema-version change before external use;
- explicit documentation of missing-value and conflict behavior;
- positive and negative fixtures;
- tests proving no arbitrary code evaluation is possible;
- review that the new field's provenance is appropriate and its privacy impact is proportionate.

The safest default for uncertain applicability is to ask the user a plain-language follow-up or direct them to the relevant authority, not to guess.
