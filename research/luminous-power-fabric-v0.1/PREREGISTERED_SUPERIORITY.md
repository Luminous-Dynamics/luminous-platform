# Preregistered lifecycle superiority gate — v0.1

**Tracking:** [Issue #466](https://github.com/Luminous-Dynamics/luminous-platform/issues/466) · [Lifecycle frontier PR #462](https://github.com/Luminous-Dynamics/luminous-platform/pull/462) · [Paired cluster bootstrap PR #463](https://github.com/Luminous-Dynamics/luminous-platform/pull/463)

**Status:** synthetic/offline confirmatory-decision screen. No field superiority or equivalence is claimed.

## Why

A nominal point-estimate win can be smaller than practical significance, disappear under meter uncertainty, or be a false positive selected from many candidate comparisons. This gate binds the existing lifecycle frontier and paired cluster bootstrap to a frozen preregistration. It does not recalculate the source measurements or bootstrap.

## Preregistration

Freeze the comparator set, reference and candidate IDs, service boundary, site/mission/scenario/initial-terminal digests, one primary metric, direction, family-wise alpha, minimum practically meaningful improvement, bootstrap algorithm/resampling unit, and paired metrology bounds before observing results.

v0.1 supports one lower-is-better primary endpoint: cost per delivered critical-service kWh, unserved critical Wh, or system-unavailable hours. Other outcomes remain descriptive until separately preregistered.

## Multiplicity

For m candidate-vs-reference comparisons and family-wise alpha alpha, v0.1 requires two-sided Bonferroni confidence 1 - alpha/m. The bootstrap report confidence must match. This is conservative and transparent, not Holm or Westfall-Young.

## Metrology uncertainty

Each candidate requires an owner-supplied paired candidate-minus-reference absolute measurement-error bound for the primary metric and all relevant POCs. v0.1 accepts only a conservative absolute bound with current evidence from independent physical measurement or analytical model, plus method and evidence references. Expanded coverage intervals, assumptions and synthetic evidence are rejected.

The gate adds the guaranteed absolute bound to both endpoints of the adjusted bootstrap interval. It does not add two probabilistic confidence intervals and pretend to preserve the same coverage. A probabilistic expanded uncertainty requires a proper joint-coverage calculation before admission.

## Decisions

- established: combined upper bound is below the negative minimum meaningful improvement.
- no_preregistered_superiority: combined interval lies entirely above the minimum-improvement threshold.
- inconclusive: combined interval crosses the threshold.
- ineligible: candidate fails the #462 service/evidence gate.

No-superiority and inconclusive do not mean equivalence. Equivalence/non-inferiority requires a separate design.

## Lineage

The lifecycle frontier report now retains service-boundary, site, mission-set, scenario-manifest and initial/terminal-obligation digests. The gate checks them against the preregistration and #463 bootstrap report, plus exact candidate family, algorithm and resampling unit.

## Synthetic fixture and claim ceiling

The fixture has two candidates, family alpha 0.05 and expected two-sided confidence 97.5%, with synthetic paired metrology bounds. It is a structure fixture, not a measured comparison.

`PREREGISTERED_SUPERIORITY_SCREEN_ONLY_NOT_FIELD_SUPERIORITY_OR_EQUIVALENCE`


## Validation status and claim ceiling

- 12 Rust unit tests are authored, not executed.
- The schema and synthetic fixture parse in the authoring runtime. The fixture has two candidate comparisons and 1 - 0.05 / 2 = 0.975 expected two-sided Bonferroni confidence.
- Authoring-time checks confirmed required fields, unique candidate IDs, one current conservative metrology bound per candidate, exact confidence arithmetic, crate module registration and the frontier context digest fields.
- No Rust compiler/Cargo, formatting, Clippy or exact-head CI result is claimed.
- The evaluator checks caller-supplied confidence intervals and bounded metrology uncertainty; it does not produce either. No statistical independence, physical superiority, product conformity, RUL, reliability or safety result is claimed.

Claim ceiling: `PREREGISTERED_SUPERIORITY_SCREEN_ONLY_NOT_FIELD_SUPERIORITY_OR_EQUIVALENCE`.
