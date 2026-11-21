# Evidence Claim Qualification Benchmark v0.1

**Status:** synthetic claim/evidence interpretation contract only. It does not verify signatures, query GitHub APIs, execute runtimes or observe resource effects.

## Purpose

ECQ is the first executable supplement for [issue #533](https://github.com/Luminous-Dynamics/luminous-platform/issues/533). Every claim is bound to an immutable subject and a required-obligation set. Evidence discharges specific obligations; missing or invalid evidence remains a visible residual gap.

ECQ consumes standard SLSA/in-toto provenance, endpoint run receipts, CARI/CAPC effects, OIC/CFI observer qualification, MTCI descriptors, TELR reconciliation and EESPE continuation proofs. It is an interpretation profile, not a new signature envelope, registry, workflow runner or policy engine.

## Standards baseline

- SLSA v1.2 is the current approved specification and defines source/build tracks and an in-toto build-provenance predicate: [SLSA v1.2](https://slsa.dev/spec/v1.2/), [build provenance](https://slsa.dev/spec/v1.2/build-provenance)
- in-toto Statement v1 binds an attestation to immutable subject digests and a predicate type: [Statement v1](https://github.com/in-toto/attestation/blob/main/spec/v1/statement.md)
- in-toto's validation model verifies a recognized attester and matches an artifact subject digest, then passes the extracted predicate to policy evaluation: [validation model](https://github.com/in-toto/attestation/blob/main/docs/validation.md)

These mechanisms establish provenance/authenticity boundaries, not automatic truth of every predicate. A signed CI result does not prove a resource-side effect; a build attestation does not prove runtime observer independence.

## Claim model

For a claim (c):
- (Scope(c)): immutable subject, repository/revision, environment, resources, operations and threat assumptions;
- (O(c)): required obligations;
- (E(c)): evidence records that discharge those obligations;
- (G(c)=O(c)-E(c)): residual obligation set;
- (A(c)): assumptions and unqualified dependencies.

ECQ deliberately reports no composite score. A critical missing obligation cannot be compensated by several weak signals.

## Profiles

- **synthetic_contract**: repository checker and mutation suite pass on the exact fixture subject.
- **exact_head_hosted**: synthetic obligations plus a completed successful workflow whose head SHA exactly matches the subject and whose required checks all passed.
- **isolated_runtime**: hosted qualification plus pinned model/runtime/tool descriptor, disposable environment, trusted subject-bound provenance and current policy/descriptor evidence.
- **independent_effect_observation**: runtime qualification plus complete resource-owner observation, observer independence/fault qualification and qualified timebase.
- **bounded_edit_contract**: synthetic checker/mutations plus complete no-replay continuation/witness proof.

Profiles are independent obligation profiles, not a single scalar ladder. Passing a synthetic profile never implies runtime or independent-effect qualification. Production assurance is intentionally unsupported in v0.1.

## Frozen 20-case panel

Four positive, twelve negative and four indeterminate cases cover:
- valid synthetic, exact-head hosted, bounded-edit and independent no-effect claims;
- wrong workflow SHA, in-progress workflow, failed required check;
- mismatched in-toto subject, invalid signature, untrusted attester;
- missing runtime identity, non-disposable runtime, descriptor drift;
- agent self-report as effect truth and synthetic-only observer evidence;
- stale/superseded policy or descriptor currentness;
- unqualified timebase;
- unsealed evidence bundle;
- missing observer-fault obligation.

A complete independent no-effect observation is a scoped negative effect claim only. It is not a statement that the agent, service or deployment is generally safe.

## Evidence interpretation rules

- Workflow pass requires completed/success, exact head SHA and the full required-check set. Queued/in-progress is unknown; a different head or failed check is invalid.
- Attestation requires signature verification, a trusted attester policy and exact subject digest. A signature from an untrusted attester is not sufficient.
- Runtime evidence requires named model/runtime/environment/descriptor identity and a disposable target.
- Currentness requires policy and descriptor revision equality and no supersession.
- Independent effect requires complete resource-side evidence, a distinct qualified observer domain and exact resource/operation scope. Agent self-report is never effect truth.
- Observer fault qualification must be an executed fault campaign, not a synthetic OIC fixture.
- Timebase uncertainty remains indeterminate when ordering is not qualified.
- A bounded continuation claim requires complete proof/witness and replay-free result.

## Claim output

Each record reports:
- status: **qualified**, **failed** or **indeterminate**;
- supported profile(s);
- discharged obligation ids;
- residual obligation ids;
- invalid evidence ids.

No weighted score, majority vote or "percentage assurance" is produced.

## Claim ceiling

The 20 fixtures and digests are synthetic. The checker inspects normalized boolean/status fields only; it does not perform cryptographic signature verification, fetch workflow runs, validate actual SLSA provenance, execute a tool/runtime, confirm observer fault domains or independently observe effects. Passing ECQ v0.1 qualifies only this synthetic interpretation contract.
