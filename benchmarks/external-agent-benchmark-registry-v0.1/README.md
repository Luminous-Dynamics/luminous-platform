# External agent benchmark registry v0.1

**Snapshot date:** 2026-11-07  
**Snapshot status:** metadata snapshot, source-byte capture incomplete  
**Format:** JSON Schema Draft 2020-12

## Purpose

This is a dated evidence registry, not a composite leaderboard. It keeps benchmark constructs, denominators, units, evaluators, run protocols, source identity and claim ceilings explicit so that unlike results are not accidentally compared.

The registry currently contains twelve records spanning:

- SRE diagnosis and mitigation (including SREGym full and Lite separately);
- SRE diagnosis-only and tool-surface studies;
- enterprise workflow execution;
- harness reliability and harness evolution;
- governance-layer enforcement;
- model investigation quality/cost.

The records are intentionally not ranked together.

## Files

- `registry.schema.json`: strict Draft 2020-12 schema. All objects reject unrecognized properties.
- `registry.snapshot-2026-11-07.json`: source-qualified 12-record snapshot.

## Critical comparison rules

1. Never compare raw percentages across different task units, cohorts, action permissions, scorers, judges or repeat protocols.
2. Preserve native scores and denominators. Keep diagnosis, recommendation, mitigation execution, participant effect and sustained recovery separate.
3. Label vendor- and benchmark-owner-published claims. They are not independent Luminous replications.
4. A benchmark's native score must remain intact. Luminous supplemental controls are separate metric families.
5. Use fault/task-level pairing and cluster-aware intervals for confirmatory comparisons. Repeated attempts within one fault are not independent fault samples.
6. Missing source bytes or content hashes remain explicitly missing. A URL, Git commit or blob identifier is not a SHA-256 content digest.
7. No composite or weighted "overall SRE score" is allowed. Safety, authority, positive-control acceptance, evidence completeness and verified effects are non-compensatory.

## Snapshot completeness

This snapshot is marked `metadata_snapshot_partial`. The following capture states are distinguished:

- `metadata_only_no_content_hash`: source inspected, but exact source bytes were not retained and hashed here.
- `raw_git_blob_sha_only`: a Git blob SHA is recorded, but this is not a SHA-256 content hash.
- `content_sha256_verified`: reserved for future snapshots where the exact captured bytes have a verified SHA-256 digest.

Do not promote this snapshot to `source_bytes_hashed_complete` until every primary source and every necessary raw dataset/scorer artifact has been captured, hashed and linked. A vendor blog does not substitute for raw task outcomes.

## Important category distinctions

- **SREGym-0508:** 90-fault full cohort, three attempts/fault; the 95.2% E2E is diagnosis and mitigation both correct on the same attempt.
- **SREGym-Lite-1004:** 17-fault Lite cohort; a smoke/regression set, not the full-suite frontier.
- **SRE-Marathon:** 41.3 is native Marathon-Score, not a repair percentage. Report active-fault repair fraction separately.
- **AI SRE Arena:** the published supported-fix measure is a recommendation score; the study did not execute the repair or verify recovery.
- **Traceway AI SRE Bench:** deterministic grading includes answer, evidence, safe writes and hidden tests; one run per case/setup does not estimate run-to-run stability.
- **ITBench-AA:** AP@FR is a diagnosis precision metric with a hard zero on false negatives; it is not a resolution rate.
- **AgentGovBench:** governance-layer scenario score; it is not SRE diagnosis or model harmful-refusal performance.
- **EvoHarnessBench:** retention/adaptation under staged harness evolution; not an SRE score.
- **Datadog Scout / AlertEval:** expert-explanation agreement and estimated inference cost; not incident-resolution percentage.

## Reproduction and updating

To update this registry:

1. Inspect the benchmark's primary paper, maintainer page, official scorer and raw dataset source where available.
2. Pin the repository commit, release or leaderboard snapshot separately from the publication date.
3. Record exact task, run and repeat counts; judge identity; tool surface; permissions; environment and retry policy.
4. Preserve reported metrics in their native units. Only derive count ratios when numerator/denominator and aggregation semantics are explicitly known.
5. Add limitations and a claim ceiling for every metric.
6. Validate the snapshot against the JSON Schema and run negative controls.
7. Review any license separately for paper, code, scenario text, task data, fixtures, dependency and result artifacts before copying or redistributing them.

## Claim ceiling

The registry improves provenance and prevents category errors. It does not certify third-party benchmark correctness, independently reproduce any external score, or establish Luminous superiority. Only a separate matched experiment can establish a comparative claim.
