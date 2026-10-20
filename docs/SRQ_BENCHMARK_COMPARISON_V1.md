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

The SQLite reference harness has passed four GitHub-hosted runner executions:

| Run | SQLite settings | Contenders | All-contender p50 / p95 / p99 | Winner elapsed |
|---|---|---:|---:|---:|
| [39431749407](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39431749407) | 3.45.1, WAL, synchronous=FULL | 64 processes | 82.1 / 138.6 / 150.8 ms | 47.4 ms |
| [39432653655](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39432653655) | 3.45.1, WAL, synchronous=FULL | 64 processes | 34.7 / 68.4 / 87.2 ms | 11.5 ms |
| [39433030759](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39433030759) | 3.45.1, WAL, synchronous=FULL | 64 processes | 48.1 / 79.2 / 94.9 ms | 42.8 ms |
| [39433712275](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/39433712275) | 3.45.1, WAL, synchronous=FULL | 64 processes | 22.0 / 57.7 / 62.3 ms | 9.0 ms |

These are per-contender script elapsed times, including CLI/process startup and lock waiting; they are not pure database commit latency. The spread is evidence of runner scheduling noise, not a SQLite performance flaw or proof of any superiority. A defensible comparison still needs a fixed machine/storage tuple, repeated randomized order, raw per-trial values, and the same fault schedule against the production Luminous adapter.

The latest tested code also binds the SQLite log and process-kill log into the deterministic SRQ archive manifest. The PR's contract tests check the fixed inventory and both digests, plus exact workflow-file SHA matching. The real GitHub/Sigstore attestation has not been minted or verified because the trusted-main job is skipped on PR events. Overall SRQ remains NOT_QUALIFIED, 0/22 gates met at qualifying tier.
