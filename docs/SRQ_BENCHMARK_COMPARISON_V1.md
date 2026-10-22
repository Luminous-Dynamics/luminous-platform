# SRQ-1 benchmark comparison protocol v1

Status: protocol and proposed gates. No SOTA-superiority claim is authorized by this document.

## Objective

Compare the Luminous cross-layer recovery path to relevant established components without pretending they solve the same problem. The unit of comparison is a defined end-to-end outcome under a frozen fault schedule, not the number of checks or lines of code.

## Baseline composition

| Layer | Reference baseline | What that baseline already contributes | What SRQ must prove beyond it |
|---|---|---|---|
| Update metadata | TUF conformance/threat fixtures | Authenticated metadata roles and protections against rollback, freeze, mix-and-match and wrong software | Correctly compose TUF verification with target/policy/generation binding and the deployed outcome; do not replace TUF semantics |
| Deployment recovery | OSTree atomic deployments and supported NixOS generation rollback | Old-or-new deployment selection and rollback behavior | Show that the intended authorized artifact is the generation actually booted, and that recovery outcome is observed after restart |
| Image/resource update | systemd-sysupdate where deployment shape matches | A/B/C resource-update mechanisms and atomic version switching | Compare only matching resource granularity and failure semantics; report unsupported cases, not synthetic parity |
| Policy promotion | SQLite transaction reference with WAL and synchronous=FULL | ACID transaction boundary, write serialization, uniqueness constraints, rollback and recovery machinery | Integrate challenge consumption, decision identity, policy digests and generation compare-and-swap into the production updater path |
| Build provenance | GitHub artifact attestations / SLSA provenance | Signed binding of artifact digest to a workflow/source identity under a verifier trust policy | Verify provenance before an artifact can count; separately verify which runtime generation is actually active |
| Runtime trust | RFC 9334 RATS terminology and freshness-policy separation | Clear separation between evidence appraisal and reliance | Enforce freshness and target binding while explicitly handling the race between evidence generation and mutation |
| Firmware resilience | NIST SP 800-193 protect/detect/recover | Firmware-specific resilience framing | Do not imply OS rollback tests qualify firmware, secure boot roots, TPM or physical recovery |

Reference links:
- TUF specification: https://github.com/theupdateframework/specification/blob/master/tuf-spec.md
- OSTree atomic upgrades: https://github.com/ostreedev/ostree/blob/main/docs/atomic-upgrades.md
- systemd-sysupdate: https://www.freedesktop.org/software/systemd/man/latest/systemd-sysupdate.html
- SQLite atomic commit: https://www.sqlite.org/atomiccommit.html
- SQLite WAL mode: https://www.sqlite.org/wal.html
- SQLite PRAGMA synchronous: https://www.sqlite.org/pragma.html
- GitHub attestation verification: https://cli.github.com/manual/gh_attestation_verify
- RFC 9334 (RATS): https://www.rfc-editor.org/rfc/rfc9334.html
- NIST SP 800-193: https://csrc.nist.gov/pubs/sp/800/193/final

## Baseline version lock and security eligibility (2026-10-22)

A comparison is not reproducible if its standards/spec pages float to `main` or `latest`, and "newest" does not mean "eligible for every threat class". `benchmarks/srq-v1/industry-baseline-candidates-v1.json` records candidate selections and known caveats, and the corpus validator checks it against the TSV matrix. It is not a per-run lock or execution evidence. Before interpreting a comparison, a separate run lock must bind the exact upstream release/tag and resolved commit SHA (or official archive digest), build/toolchain, configuration, artifact digest, OS/kernel/storage tuple, fault corpus digest, fault schedule and excluded threat classes. The lock digest must be copied into each comparative run receipt; absent baseline-lock identity means `NOT_COMPARABLE`, not a performance win or loss. The current receipt schema does not yet enforce a comparative-lock digest, so that integration remains an explicit open gate.

The upstream sources checked on 2026-10-22 identify these candidate baselines:

- **TUF:** the releases page lists stable release `v1.0.36`, while the upstream working specification advertises `1.0.37`. Use the published/tagged `v1.0.36` for the frozen stable comparison and record the floating development branch separately if it is also examined. Do not label a development spec as a released conformance baseline. Sources: [release/tag](https://github.com/theupdateframework/specification/releases/tag/v1.0.36), [working specification](https://github.com/theupdateframework/specification/blob/master/tuf-spec.md).
- **OSTree:** the releases page lists `v2026.4` as latest. Its release notes explicitly say the change removes the 512 MiB static-delta part cap and reopens `GHSA-7cgc-gp99-6jmm` pending an exact-size replacement. Therefore the benchmark must declare whether it tests only trusted/prevalidated deployment objects or includes hostile static-delta ingestion. A run that excludes that ingestion path must say so; it cannot support a claim about resilience to hostile delta payloads. Keep the known advisory as an explicit negative control in a separate sandboxed threat campaign. Sources: [v2026.4 release](https://github.com/ostreedev/ostree/releases/tag/v2026.4), [atomic-upgrade contract](https://github.com/ostreedev/ostree/blob/v2026.4/docs/atomic-upgrades.md).
- **systemd-sysupdate:** the stable release list includes `v261.4` (2026-10-17). Pin the source/manual to that tag for the first comparison rather than `man/latest`. Source: [systemd-stable v261.4](https://github.com/systemd/systemd/releases/tag/v261.4).
- **SQLite:** use a release whose engine fix includes the WAL-reset correction: upstream documents the issue through `3.51.2`, fixed in `3.51.3` and later. The branch's reference harness selects SQLite `3.54.1`; every receipt must bind both its source identity and runtime `sqlite_source_id()`. Source: [SQLite WAL documentation](https://www.sqlite.org/wal.html).

These are source-review observations and baseline-selection rules, not executed comparator results. Before a claim of superiority, freeze the exact tag commit and configuration in a machine-readable lock file. For standard documents without executable release tags (e.g., RFC 9334 and NIST SP 800-193), pin the document identifier/revision and the appraisal/control profile, not just a vendor's `latest` landing page.

## Machine-enforced per-run comparison lock v1

A baseline registry is only a selection catalog. A comparative scenario receipt must carry an explicit `comparison_lock` with `baseline_id` and SHA-256 of the exact lock-file bytes. When this field is present, `validate-result.sh` requires the lock file, invokes the lock validator, verifies its bytes, requires its baseline ID to match, and binds the receipt's expected and observed SRQ source to the locked SRQ commit.

The lock pins the baseline's resolved upstream commit or official source-archive SHA3-256 (including SQLite `sqlite_source_id()` when relevant), exact SRQ source SHA, benchmark artifact, configuration, corpus, fault schedule, outcome-oracle digests, OS/platform/storage/runner tuple, and included/excluded threat scope.

Only registry entries marked `pinned_reference_not_executed` can be locked for a comparison. Moving, unknown, unpinned, protocol-only or scope-blocked baselines fail closed. OSTree v2026.4 must not be used for hostile static-delta comparisons until its caveat is resolved or the scope is explicitly reviewed and registered as eligible.

The committed lock fixture is accepted only with `--allow-fixtures`. Lock validation establishes structure and registry correspondence, not execution or safety. A comparative `PASS` receipt still needs authenticated provenance and raw matched-trial evidence. Non-comparative single-scenario receipts may omit `comparison_lock`.

## Matched fault-injection methodology

For every comparison, publish these inputs before running:

- Source commit(s), package/artifact digest(s), schema/metadata version, policy/corpus digest, and exact baseline version.
- OS/kernel, filesystem, SQLite/VFS settings where applicable, architecture, storage device or VM backend, and runner/hardware class.
- Frozen random seed(s), number of attempts, fault injection points, and exact fault mechanism (ordinary function return, SIGKILL, VM reset, kernel panic, or physical power interruption).
- A shared outcome oracle that validates the complete generation, decision ID, challenge ledger, policy digests, recovery boot selection and data-loss state.

Do not compare:
- early-return models against SIGKILL runs;
- SIGKILL runs against kernel crash or sudden-power-loss claims;
- QEMU results against physical hardware;
- model-tier receipts against a QEMU/physical gate;
- TUF metadata verification against a runtime measured-boot claim;
- a production database transaction against a synthetic single-file rename and combine them into one headline score.

## Required measurements

Publish the distributions and denominators, not only averages:

1. Safety outcomes: unauthorized accepts, replay accepts, hybrid/inconsistent recovered states, duplicate committed decisions, false rejects and unclassified faults. Any unauthorized accept or hybrid state is a release blocker, not a latency trade-off.
2. Durability outcomes: complete old state, complete new state, recoverable but blocked, state lost, state inconsistent, and unavailable/unobserved. Keep timeout/unreachable separate from safe recovery.
3. Timings: p50/p95/p99 from request to decision, durable commit, caller acknowledgement, fresh-process reopen, and return to defined safe operation.
4. Resource and operational cost: CPU/memory/I/O, retry count, log/evidence volume, operator handling time and task errors. Specify if measurement is unavailable.
5. Coverage: trial count by each fault class, exact execution tier, repeated runs and every excluded case.

For latency percentile calculations use an explicit quantile convention (nearest-rank for these small test harnesses is acceptable); retain raw per-trial values. Do not report percentiles from only successful cases without also reporting failure counts and rejected/timeout outcomes.

## Initial gates and statistical meaning

These are engineering targets; they are not achieved results:

- 100,000 deterministic adversarial mutations with zero unauthorized acceptance before a production security claim.
- 64 independent contenders for one target/challenge: exactly one durable committed decision; replay after reopen must be rejected.
- At least five independent timing runs for a first baseline; grow to 100,000 mutation cases and 1,000 VM interruption trials as the harness stabilizes.
- Every pre-commit process-kill case must yield no committed transaction; commit-before-ack termination must return the already committed result by decision ID, never create a second promotion.
- A 1,000-trial zero-failure VM campaign is evidence, not proof: under independent identical-trial assumptions, the approximate 95% upper bound on the true failure probability is still about 0.3%. Real trials are correlated and fault distributions are not uniform, so publish the limitations.
- Sovereign Boot keeps the existing proposed QEMU p95 ≤750 ms DRM-release target, with physical outcomes separate.
- At least 95% correct blinded operator status classification, zero NOT_RUN-as-PASS decisions, and zero mutations after cancellation or without current target-bound authorization.

## How an integration win should be demonstrated

The defensible win is not that Luminous replaces TUF, SQLite, OSTree or RATS. It is that a verified authorized decision can be followed across the layer boundaries and independently reconciled to the generation and recovery state observed after restart, while preserving trustworthy evidence of failures.

To claim improvement:
1. Run baseline and Luminous on the same test inputs and comparable hardware tier.
2. Show at least one predeclared outcome where the integrated path adds value—such as detecting mismatch between authorized generation and actually booted generation—that the narrower baseline does not claim to cover.
3. Demonstrate no regression on inherited baseline properties.
4. Publish raw results, versions, fault schedule, exclusions and confidence limitations.
5. Leave every unsupported firmware, TPM, cross-host or physical durability claim unqualified.

SRQ aggregate status remains NOT_QUALIFIED until every required scenario has evidence at its declared qualifying tier and claim ceiling.


## First observed measurements (exploratory, not a SOTA comparison)

The SQLite reference harness has passed five GitHub-hosted runner executions:

| Run | SQLite settings | Contenders | All-contender p50 / p95 / p99 | Winner elapsed |
|---|---|---:|---:|---:|
| [39431749407](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39431749407) | 3.45.1, WAL, synchronous=FULL | 64 processes | 82.1 / 138.6 / 150.8 ms | 47.4 ms |
| [39432653655](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39432653655) | 3.45.1, WAL, synchronous=FULL | 64 processes | 34.7 / 68.4 / 87.2 ms | 11.5 ms |
| [39433030759](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39433030759) | 3.45.1, WAL, synchronous=FULL | 64 processes | 48.1 / 79.2 / 94.9 ms | 42.8 ms |
| [39433712275](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39433712275) | 3.45.1, WAL, synchronous=FULL | 64 processes | 22.0 / 57.7 / 62.3 ms | 9.0 ms |
| [39445482359](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39445482359) | 3.45.1, WAL, synchronous=FULL | 64 processes | 30.6 / 50.9 / 54.6 ms | 26.4 ms |

These are per-contender script elapsed times, including CLI/process startup and lock waiting; they are not pure database commit latency. The spread is evidence of runner scheduling noise, not a SQLite performance flaw or proof of any superiority. A defensible comparison still needs a fixed machine/storage tuple, repeated randomized order, raw per-trial values, and the same fault schedule against the production Luminous adapter.

Run 39445482359 also passed 100 actual SQLite process-kill trials (25 at each of four transaction boundaries) and the mismatched target/generation negative control. This is process-death evidence on a running Linux kernel only, not kernel-crash, VM-reset or physical-power-loss qualification. The code binds the SQLite and process-kill logs into the deterministic archive manifest, with byte-digest and exact workflow-SHA controls. The real GitHub/Sigstore attestation has not been minted or verified because the trusted-main job is skipped on PR events. Overall SRQ remains NOT_QUALIFIED, 0/22 gates met at qualifying tier.


## Cross-layer boot reconciliation — first executed mutation campaign

The dependency-free Rust contract model joins promotion intent, the durable decision/consumed challenge, a separately issued post-commit observation challenge, and the observed booted deployment. It now keeps the logical SRQ generation counter distinct from an immutable deployment identity. On NixOS, the intended adapter must preserve the distinction between `/run/booted-system` and `/run/current-system`; a live userspace switch cannot masquerade as a reboot into the approved system.

The exact-head [run 39470466328](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39470466328) passed on source `59d6774ed3d3e67b2be7327aa06b1788b9d3aa5b`: 21 Rust tests, including 100,000 cross-layer mutations, 10,000 boot-observation mutations, immutable NixOS/OSTree deployment ID validation, and independent durable-record/challenge/booted-vs-current mismatch tests. The archive verifier also rejects wrong supplemental-log digests.

This improves the contract by making a cross-distro generation-number assumption explicit, not by proving real boot state authenticity. Challenge freshness, signer/device identity and truth of the boot observation remain caller trust responsibilities; QEMU or physical behavior is not tested by the Rust model. The aggregate remains `NOT_QUALIFIED`, 0/22 gates met at the declared qualifying tier.

## Transaction-binding and interrupted-commit matrix

The SQLite reference schema encodes target/decision/generation correspondence with composite foreign keys. The exact-head [run 39445482359](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39445482359) passed the mismatched target/generation negative control and the 100-trial kill campaign: 25 fresh-database trials at each boundary. Every pre-commit trial reopened the complete old state with no decision/consumption rows; every post-commit trial reopened one complete outcome by decision ID.

The tested matrix and remaining tiers are:

| Fault point | Required reopened outcome |
|---|---|
| After decision-row insert, before challenge write | Generation 0; zero decision and consumption rows |
| After challenge-consumption insert, before target update | Generation 0; zero decision and consumption rows |
| After target update, before COMMIT | Generation 0; zero decision and consumption rows |
| After COMMIT, before caller acknowledgement | Generation 1; exactly one decision/consumption; decision ID resolves to the durable outcome |
| Wrong target binding between decision and consumption | SQL constraint failure; no rows or generation changes survive |

These are process-kill tests on a running kernel. They do not establish kernel-crash, VM-reset, physical-power-loss durability or the production adapter's behavior. The workflow's zero-failure outcomes demonstrate deterministic fault-point coverage, not an estimate of field failure probability.


## Verifier-to-store composition gate

A new external-crate integration test (`tools/srq-d1-sqlite-store/tests/verified_consumption.rs`) now creates a real Ed25519-signed COSE/EAT observation and calls the public `verify_and_consume` API with `SqliteD1Store`. It checks that:
- a mutated signature fails and leaves both durable receipt and consumed-challenge counts at zero;
- the valid signed token consumes the registered target-bound challenge once;
- the exact receipt survives a fresh store reopen and integrity checks;
- replay of the same valid token is rejected after reopen.

The test passed on exact tested code head [a66ebfc6e6e03f34d08a58e48eee07843b1742d4](https://github.com/Luminous-Dynamics/luminous-platform/commit/a66ebfc6e6e03f34d08a58e48eee07843b1742d4) in [workflow run 39645017004](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39645017004). The latest documentation-only head [ab790de72f86f11566118559ac8309ebc3e4856f](https://github.com/Luminous-Dynamics/luminous-platform/commit/ab790de72f86f11566118559ac8309ebc3e4856f) also passed in [run 39645077549](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39645077549). Both runs preserve the NOT_QUALIFIED (0/22) outcome and skip real signing on pull-request events. This test remains software-key/model-tier evidence and does not qualify D2 hardware measurement, secure boot, TPM, or power-loss durability.


## Database-engine version is part of the benchmark tuple

SQLite's official WAL documentation records a rare WAL-reset corruption bug affecting SQLite releases through 3.51.2 and fixed in 3.51.3. The earlier SRQ reference timings used Ubuntu's system CLI 3.45.1; although the race produced one winner and passed integrity checks, those runs used a version in the documented vulnerable range. Retain them only as historical smoke observations, not as clean benchmark data.

The revised workflow compiles SQLite CLI 3.54.1 from the upstream autoconf source, checks the published SHA3-256 and exact source ID, and makes both SQL reference scripts reject a different engine on PATH. The Rust D1 and policy-promotion stores accept only the upstream-documented fixed WAL-reset lines (3.44.6+, 3.50.7+, 3.51.3+, 3.53.0+) and explicitly reject the withdrawn 3.52.0 line. The store contracts also re-check expiry immediately before commit and test rollback when the authorization expires during transaction execution. Record `sqlite_version()`, `sqlite_source_id()`, journal mode, synchronous mode, host kernel and filesystem with every sample. Never combine old-version numbers with the new pinned-engine series.


## Policy-promotion transaction prototype (design target, not yet qualified)

A separate Rust SQLite prototype now models the missing promotion boundary: verify a strict Ed25519 COSE authorization, re-read the target and one-use challenge under BEGIN IMMEDIATE, insert a decision, compare-and-swap the prior generation/artifact/policy/corpus/deployment, consume the challenge once, then commit. The decision, challenge pointer and active target state are reconciled after reopening. A private verified-authorization type prevents call sites from bypassing token verification by constructing trusted fields directly.

The intended new fault schedule kills a real child process after decision insertion, after target compare-and-swap, after challenge consumption but before COMMIT, and after COMMIT but before acknowledgement, 20 trials per boundary. The test must produce old complete state/no decision for the three pre-commit points and one recoverable committed result for post-commit-before-ack. These are Linux live-kernel SIGKILL tests, not power-loss tests. The log and Cargo.lock are being added as digest-bound supplemental observations, explicitly model-tier; they do not alter the 22 release gates.

The prototype is not yet integrated into the deployed updater, is not yet confirmed passing on the newest workflow head, and does not establish a performance win against SQLite itself. The fair next benchmark compares the production adapter and the SQLite reference on the same machine/storage, nonce/target corpus, 64-way race schedule, 80 per-boundary kill schedule and raw p50/p95/p99 timing series. Any test that treats arbitrary SQLite errors as a rejected contender is invalid—the test must distinguish constraint/staleness errors from I/O, corruption or unexpected database faults.


## Actual policy-store → signed boot-receipt integration (new test, awaiting exact-head CI)

The D1 SQLite store now exposes `committed_observation_for_challenge()`: it reads the receipt back from the durable database after commit and checks the persisted target, audience, decision, generation, artifact/effective-policy/corpus digests, deployment, signer, nonce digest and freshness timestamps against the issued challenge. It deliberately does not reconstruct the response solely from caller-held request fields and never returns the raw nonce.

A new external Rust integration test, `tools/srq-d1-sqlite-store/tests/cross_layer_reconciliation.rs`, composes the real Ed25519-signed policy-promotion flow, reads its committed decision back from SQLite, then verifies and consumes an actual COSE EAT boot-observation token through the public verifier API and D1 `ChallengeConsumer`. It reads the committed boot record back out of the second database and feeds those durable records into the cross-layer reconciliation oracle. Negative controls mutate the observed boot generation, deployment, durable decision ID and effective-policy digest and require fail-closed reconciliation.

This is the next meaningful boundary test: it connects the *actual* durable decision and *actual* persisted signed observation, rather than synthesizing both as independent structs. Its result still has a strict ceiling: the observation is D1/software-key only, does not verify TPM or secure-boot measurements, does not prove what a physical bootloader executed, and does not qualify QEMU/physical recovery. The reconciliation schema was normalized to match the durable policy store: the promotion record has one `approved_artifact_digest`, one `effective_policy_digest`, and one `corpus_digest`; the boot observation's separate `artifact_digest` is compared to the approved artifact digest. This removes a prior duplicated-field projection and makes the comparison's field semantics explicit.

Exact-head execution is pending. Until the Rust integration test runs on the complete current head, do not count it as passed evidence or alter the `NOT_QUALIFIED`, 0/22-gate result.
