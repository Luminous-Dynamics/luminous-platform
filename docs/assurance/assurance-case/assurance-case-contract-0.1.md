# Executable assurance case contract 0.1

**Status:** initial synthetic reference evaluator; not a certification scheme  
**Parent:** [ASSURANCE-CASE-001 / Platform #345](https://github.com/Luminous-Dynamics/luminous-platform/issues/345)  
**Benchmark:** [OMSI-EVAL-001 / Platform #325](https://github.com/Luminous-Dynamics/luminous-platform/issues/325)

## Why this exists

Artifact validation tells us whether bytes, schemas, provenance and receipts are consistent. It does not by itself establish why those artifacts justify a particular claim. An assurance case is the argument connecting claims, context, assumptions and evidence. The 15026-2:2022 standard defines assurance-case structure and terminology, but not the quality of each case's content. This v0.1 contract adds executable checks for a limited subset of content obligations: required evidence, time validity, scope, dependencies, assumptions, defeaters and rebuttals.

## Node vocabulary

- **Claim:** proposition with a declared decision scope and required support.
- **Argument:** explicit rationale node connecting evidence to a claim.
- **Context:** system boundary and operating conditions.
- **Assumption:** premise with supported, bounded, open or refuted state.
- **Evidence:** exact artifact reference, source/provenance, evidence class, scope, temporal validity and optional dependency assessment.
- **Defeater:** applicable counter-condition that undermines a claim or argument.
- **Rebuttal:** evidence-backed response to one exact defeater. It does not erase the defeater's history.

## Decision rules

1. Required evidence must be connected to its claim and valid at the case's declared `as_of` time.
2. A critical open defeater defeats the claim until an evidence-backed rebuttal addresses that exact defeater.
3. An open assumption is indeterminate; a refuted critical assumption defeats the claim.
4. A revoked, stale, conflicted, missing, unavailable, out-of-scope or expired evidence item cannot satisfy a required obligation.
5. A shared dependency defeats a claim that requires independent corroboration.
6. An unknown dependency is indeterminate.
7. `NO_DECLARED_COMMON_DOMAIN` can support only the narrow proposition that a complete declared inventory contains no common dependency. It cannot establish physical or statistical independence.
8. Structural graph faults (dangling edges, invalid endpoint types, disconnected claims, wrong rebuttal bindings, cycles) produce `INVALID_ARGUMENT_GRAPH`.
9. A positive outcome means only `SUPPORTED_FOR_DECLARED_SCOPE` for the declared case and time; it is not a universal trust score, certification or proof of physical truth.

## Reproducibility

The evaluator binds the report to the exact input bytes with SHA-256. The report exposes per-claim outcomes, open defeaters, unresolved assumptions, reasons and claim ceiling. It does not authenticate source signatures or custody receipts; those remain owned by the existing provenance and sealed-evaluation layers.
