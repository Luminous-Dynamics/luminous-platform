# SRQ Lifecycle Interoperability v1

**Status:** research conformance smoke, not a release qualification.

This work is intentionally narrow: prove that the existing Nixward transaction envelope, Sovereign Boot's actual D0 diagnostic, and Mycelix's shared evidence-plane planned receipt can be consumed together without silently increasing the strength of any claim.

## Why this is the next paradigm shift

Each repository has a useful local contract, but individually valid records do not automatically compose into a valid lifecycle decision. A transaction can be merely planned, an observation can be local and unauthenticated, and an evidence receipt can be synthetic and not run. Combining them must preserve those limits instead of creating a false green result.

The test uses immutable upstream commit pins and exact Git blob IDs rather than branch names or copied-upstream files:

- Nixward transaction fixture: commit  ae60256f0be73138a539ecda048e81056dd14a30; blob c89d5033a35e5c1be1b5c7de0a2436539922a800.
- Mycelix evidence-plane schema: commit b02cefab8474262aef23b61a4ac3811fe92e29bf; blob e2b90a995f70d0a771b04e7f5fc077e1c0394f86.
- Mycelix planned receipt: the same pinned commit; blob d1d3516e2778e0da89013348b16ed73af6e5e8a8.
- Sovereign Boot D0 probe source: commit 9d183163a0f2bb101e6dd0ad3609afde04f1c5a4; blob 2b05462ab98e90abc085d2834e69dbf2ebae9337.

The workflow checks the checkout SHA and Git blob ID at runtime and computes SHA-256 over the exact raw fixture/observation bytes for the assessment report. The primary assessment binds the exact lifecycle-interoperability harness commit, validator/test/adversarial-runner identities, full-schema validator and its unit-test source, version-pinned schema-validator dependency list, upstream-lock bytes, assessment-schema bytes, and the frozen corpus manifest; it refuses to run if tracked harness inputs have staged or unstaged edits. The adversarial runner emits a JSON assessment with every mutation filter, candidate raw-byte SHA-256, schema-validation result (or explicit `NOT_APPLICABLE` for Nixward transactions), schema-report digest, expected/observed outcome, validator exit status, rejection reason, and harness/fixture identities. CI compares these outcomes against `corpus-v1.json` and retains that versioned manifest alongside the assessments and per-case schema reports as artifacts.

The protocol manifest is versioned, Git-blob-addressed, and SHA-256-bound, but it is not separately signed or externally timestamped; this provides reproducible same-corpus comparisons, not cryptographic proof of pre-registration. The standalone `corpus_validate.py` validates the manifest against `corpus-v1.schema.json` and independently checks the exact v1 case/control ID inventory, allowed schema/lifecycle outcomes, tier/claim-ceiling and transaction-phase mappings, zero-error targets, release-blocked invariant, and exact upstream pin equality against `upstream-lock-v1.json`. Its 12 focused tests mutate duplicate/missing/replaced IDs, release and claim ceilings, upstream digests, metrics, dispositions, positive-control mapping, and protocol identity.

The separate `corpus-executable-mutations-v1.json` companion stores the actual `jq` transformation for each case and positive control; the runner no longer keeps those transformations as shell literals. CI requires a one-to-one ID/input-kind/disposition join to the parent frozen protocol and binds the executable manifest's Git blob and raw-byte SHA-256 into the primary and adversarial reports. This prevents a case ID from silently pointing at a different mutation without a corresponding manifest change. The transformations themselves remain executable test code and require review; digest binding is not a signature or external preregistration proof.

The phase-matrix oracle is separately specified in `transaction-phase-state-protocol-v1.json`: six ordered presence bits, nine phases, a required-record set per phase, the minimum-presence rule, and fixed expected counts of 576 total states / 175 accept / 401 reject. The matrix runner derives expected outcomes from this file rather than duplicating the phase rules in shell, and binds its Git blob and SHA-256 into both the matrix report and the primary assessment. The manifest itself is preflight-checked for the exact v1 phase/record inventory and zero-error targets. This makes the expected result inspectable and reusable by alternate implementations; it remains a test oracle, not a proof that the production state machine is complete.

The separate schema-test environment is itself pinned by a 20-wheel SHA-256 lock, with Python version and Linux architecture checked at runtime. CI retains pip's own install report, the repository environment lock, and a validator report listing the exact installed wheel identities. This makes dependency resolution fail closed on unexpected packages, missing packages, version changes, wheel substitutions or hash mismatches. It is a narrow test-only reproducibility measure, not an SBOM security audit or proof that any dependency is benign.

## Executable contract

The benchmark script:

1. Checks and runs the actual pinned Rust D0 probe with `cargo fmt --check` and `cargo test --locked`.
2. Builds a disposable filesystem fixture and captures the probe's actual JSON output; it does not maintain a hand-copied diagnostic output fixture.
3. Validates the pinned Mycelix receipt with the full JSON Schema Draft 2020-12 meta-schema and instance validator, with format assertions enabled, using a test-only Python environment. The test environment is scoped to Ubuntu 24.04 / Linux x86_64 / CPython 3.12. `requirements-schema.txt` pins every package version and wheel SHA-256; CI installs only wheels with pip `--require-hashes --only-binary=:all:`. `schema_env_validate.py` checks pip's install report against `schema-env-lock-v1.json`, including package set, versions, wheel filenames and hashes, and the environment validation report is digest-bound into the primary assessment. The schema validator rejects duplicate JSON keys and remote `$ref` values and never makes a release decision.
4. Runs focused unit tests for duplicate-key and non-standard-number rejection, no-follow symlink handling, bounded reads, local versus remote references, format discovery and JSON Pointer escaping.
5. Checks the additional safety-critical Mycelix cross-field invariants and Nixward transaction phase/terminal-disposition prerequisites.
6. Runs the original D0 authentication/qualification and S0-to-S4 claim-escalation controls.
7. Reads `corpus-v1.json`, validates it against the versioned full JSON Schema, and runs the independent semantic corpus validator. The validator checks exact v1 IDs and expected dispositions, rejects duplicate/missing/replaced cases, confirms upstream pins agree with the immutable checkout lock, and checks the zero-error targets and release invariant. The protocol, schema, validator and validator tests are hash-bound into the assessment.
8. Loads executable transformations from `corpus-executable-mutations-v1.json`, verifies their one-to-one join to the parent protocol, and executes the manifest-defined negative and positive controls by ID. It records the exact transformation string in each case record and requires that record to match the digest-bound manifest.
9. Runs seventeen named adversarial cases: nine Mycelix receipt mutations (run status, claim ceiling, source/gate/hash consistency and unknown tiers), six Nixward phase-only mutations (`validated`, `authorized`, `snapshotted`, `applied`, `verified`, `promoted`), and two invalid terminal-disposition mutations (`failed` without an outcome record and `recovered` without snapshot/outcome evidence). All nine receipt mutations must fail full-schema validation and the lifecycle adapter; transaction cases report schema applicability as `NOT_APPLICABLE`.
10. Accepts five explicitly synthetic positive-control shapes: Mycelix S2/source-reproducible and S4/domain-validated shapes, plus Nixward `validated`, `failed` and `recovered` shapes. Valid outcomes are compared with the frozen parent manifest and the executable transform manifest, and every control must still produce `release_status=BLOCKED` and `claim_ceiling=none`.
11. Reads `transaction-phase-state-protocol-v1.json` as the independent phase/evidence-presence oracle, verifies its v1 inventory and zero-error target values, then enumerates nine phases × 64 presence/absence combinations: 576 states, expected 175 accept and 401 reject. Each candidate is run through the lifecycle adapter; each candidate's raw bytes are hashed, the Nixward source fixture is pinned, and the report's protocol ID/blob/SHA-256 and measured counts must match the manifest. The run requires zero false accepts, false rejects, unexpected validator errors or false release passes.
12. Confirms the upstream fixture files and probe source remain unmodified.

The Mycelix receipt is now checked against the actual pinned Draft 2020-12 schema, while the additional Mycelix cross-field check remains a deliberately small safety-critical subset. The schema runner checks the schema itself against the Draft 2020-12 meta-schema, enables format assertions, and prohibits remote references. It is an isolated test-only dependency and not a production runtime path. Version pinning does not equal hash-locking the package artifacts, and a schema-valid record still does not establish the truth/authenticity of its source bytes, rights, execution or domain claim.

Positive-control URLs, hashes, commits, verifier references and transaction records are sentinels; they exercise structural acceptance only and do not establish actual source ingestion, rights, execution, independent verification or domain truth. Every mutation uses disposable output; upstream source bytes and pins remain unchanged.

The schema validator does not replace Nixward's full production transaction verifier, SRQ's release evaluator, or a cryptographic attestation verifier. JSON Schema checks structural validity; policy, provenance authenticity, actual target state and qualification remain separate evidence dimensions.

## Required result for this scenario

The integration assertion must distinguish these outcomes:

- contract_status = PASS means only that the pinned example records are structurally consistent with this narrow interoperability contract.
- release_status = BLOCKED is required because the Nixward record is planned; the Sovereign Boot record is D0_LOCAL_DIAGNOSTIC, unauthenticated and has no target/decision/challenge/policy/artifact/measurement binding; and the Mycelix receipt is S0 / synthetic_only / planned / not_run.
- The harness must never issue SRQ PASS. A separate qualifying-tier scenario evaluator must consume authenticated, source-bound, actually executed evidence before any release claim is considered.

A matching /run/booted-system and /run/current-system path still does not prove that the intended update was authorized, that the booted artifact was measured, or that recovery has been qualified.

## Industry alignment and intended contribution

This is a composition test, not a new attestation or deployment format. It follows the separation of evidence and appraisal described by [RFC 9334 / RATS](https://www.rfc-editor.org/rfc/rfc9334.html), software artifact provenance described by the [in-toto Attestation Framework](https://github.com/in-toto/attestation), and the [SLSA provenance predicate](https://slsa.dev/provenance/v1). For update security, [TUF](https://theupdateframework.github.io/specification/v1.0.37/index.html) already addresses repository threats including rollback, freeze, and mix-and-match attacks. For transactional deployment behavior, [OSTree's atomic-upgrade model](https://ostreedev.github.io/ostree/atomic-upgrades/) explicitly aims to leave the old or new bootable deployment after a crash or power interruption. Existing project wire formats remain authoritative for their own domains.

The competitive opportunity is the cross-layer guarantee: join authorization, transaction state, measured boot/runtime evidence, independent appraisal, rollback outcome, and claim ceiling without allowing a lower layer's "green" status to imply the whole system is safe. This harness does not yet prove that end-to-end guarantee or reproduce TUF/OSTree guarantees on NixOS; future fault-injection work must test these boundaries directly.

The value hypothesis is operational: catch trust-tier escalation and contract drift across repository boundaries before the components are deployed together. The success measure is 100% rejection of the frozen claim-escalation controls, byte-identical upstream inputs at their locked commits, and no false release PASS in the integration corpus.

## Benchmark ladder: targets, not results

These are proposed gates for follow-on work, not results measured by this PR. Start with deterministic attacks and simulated recovery; only graduate to physical/fleet claims when those environments have been exercised.

| Benchmark | Industry reference / fair baseline | Target |
|---|---|---|
| Frozen-protocol reproducibility | `corpus-v1.json` + strict corpus schema + independent semantic validator | Every implementation run must match 17 negative and five positive case IDs, schema dispositions and expected lifecycle outcomes; reject duplicate, missing or substituted IDs; zero manifest/report mismatches. |
| Executable-transform reproducibility | `corpus-executable-mutations-v1.json` companion + parent-manifest join | Each case ID maps to exactly one frozen transform and disposition; zero missing, duplicate or mismatched transforms; each reported transform string equals the hashed companion manifest. |
| Test-environment reproducibility | `schema-env-lock-v1.json` + pip `--require-hashes` | Exactly 20 approved wheels, every version/filename/SHA-256 matched; unsupported Python/platform, package drift or hash mismatch must fail closed. |
| Full schema conformance | Pinned Mycelix Draft 2020-12 schema + `jsonschema==4.26.0` | Reject all frozen invalid receipt fixtures; accept all known-good receipt controls; pass meta-schema and format assertions; zero false release PASS. |
| Transaction phase consistency | Nixward transaction v1 phase sequence, including failed/recovered terminal dispositions | Reject 100% of phase-only advancement attacks and invalid terminal dispositions; accept all frozen valid structural controls; zero false release PASS. |
| Phase/evidence state-space conformance | `transaction-phase-state-protocol-v1.json`; nine phases × six presence bits | Exercise all 576 states against the digest-bound minimum-prerequisite oracle: expected 175 accept / 401 reject; zero false accepts, false rejects, unexpected errors or false release passes; zero protocol/report mismatches. Structural only; nested semantics and physical recovery remain untested. |
| Claim-ceiling monotonicity | Mycelix receipt rules; RFC 9334 evidence appraisal | Reject 100% of pre-registered escalations; zero false release PASS; zero false rejects across explicit valid controls. |
| Source and report identity | Git object IDs plus raw-byte SHA-256; in-toto subject binding | Reject 100% of wrong-commit, wrong-blob, one-byte-drift, symlink-substitution, and artifact/subject mismatch cases. |
| Freshness and replay (future D1 verifier) | RATS freshness requirements and TUF rollback/freeze/mix-and-match threat classes | Reject every stale/replayed nonce, wrong target, wrong policy digest, and wrong artifact binding; under 64 concurrent submissions of one single-use challenge, exactly one acceptance. |
| Crash/rollback consistency | [OSTree atomic-upgrade behavior](https://ostreedev.github.io/ostree/atomic-upgrades/) and NixOS generations | Inject 100 process kills at each durable transition boundary, then 1,000 VM reset/restore trials; observe old or new known-good state or an explicitly blocked recovery—never silent partial promotion. Report QEMU and physical-device outcomes separately. |
| Independent appraisal | in-toto authenticated attestations, SLSA provenance, RATS verifier/relying-party split | Reject invalid signatures and policy/target mismatches; any verifier disagreement blocks promotion; record verifier implementation identity and replayed commit. |
| Fleet behavior | TUF client/update safety plus atomic deployment mechanisms | Exercise 100, 1,000, 10,000, then 100,000 simulated targets; report p50/p95/p99 detection, promotion and rollback latency, throughput, peak memory, retries, and maximum failure blast radius. No fleet-safety claim from simulation alone. |

Benchmark integrity requirements: freeze and hash the attack corpus before running; pin baseline revisions; include both valid and invalid controls; report exact hardware/runtime and resource budgets; retain raw per-case outcomes and tool identities; separate test-harness self-consistency from externally verified evidence. A high rejection percentage is meaningless if valid controls are also rejected, and speed is not a substitute for recovery correctness.

## Non-claims

- No NixOS build, bootloader, QEMU, TPM, physical reset, power-loss, or production fleet qualification.
- No cryptographic validation of the D0 JSON; the probe explicitly produces non-authenticated diagnostics.
- No Mycelix source-truth, rights, stewardship, independent-reproduction, or domain-validation claim.
- No claim that a Git digest proves authorization or semantic correctness.
- No performance/SOTA claim and no assertion that a workflow being queued or passing on a later commit qualifies the target system.
