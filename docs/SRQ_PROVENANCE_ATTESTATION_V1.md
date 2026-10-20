# SRQ-1 authenticated evidence provenance v1

**Status:** implementation proposal under review; not yet exercised by a trusted-main workflow run. The release qualification profile remains independent and fail-closed.

## Why a signature layer is necessary

The receipt validator and aggregate evaluator already check source SHA fields, execution tiers, claim ceilings, duplicate scenario/tier receipts, and the SHA-256 of referenced evidence bytes. Those checks do not prove who created a JSON receipt and its matching log. Anyone able to supply both bytes and their digest can satisfy a byte-integrity check.

This work introduces a signed, digest-bound evidence bundle and an explicit trust policy. The trust mechanism is GitHub artifact attestation/SLSA provenance. The GitHub CLI is pinned to version 2.102.0 and its Linux x86_64 archive is verified against a repository-pinned SHA-256 before use. The signing action is pinned to its full commit SHA.

## Trust policy

The verifier accepts only all of the following:

- Repository: `Luminous-Dynamics/luminous-platform`.
- Signer workflow: `Luminous-Dynamics/luminous-platform/.github/workflows/srq-v1.yml`.
- OIDC issuer: `https://token.actions.githubusercontent.com`.
- Predicate type: `https://slsa.dev/provenance/v1`.
- Runner class: GitHub-hosted runner only; the verifier invokes `gh attestation verify --deny-self-hosted-runners`.
- Source ref: `refs/heads/main`.
- Exact expected source commit SHA and the SHA-256 digest of the bundle being verified.
- Workflow-produced run ID and attempt match the manifest inside the signed subject.

The signing job runs only for `workflow_dispatch` on `refs/heads/main` and only after the full corpus, model, and negative-control job succeeds. The PR validation job remains read-only and does not receive OIDC/attestation write permissions. Repository rules must protect the main branch and its workflow file; provenance verification does not independently prove that branch-protection rules were configured.

## Evidence flow

1. The validation job gathers the two model logs, two model result receipts, and the release-evaluation report. It writes a checksum manifest and a run/source/corpus manifest.
2. GNU tar/gzip create a deterministic archive containing a fixed inventory of those files, the checksum manifest, and the run manifest.
3. On an explicit manual run of protected main, a separate job downloads the immutable Actions artifact and validates its exact file inventory, safe paths, checksum list, byte hashes, repository/workflow identity, run ID/attempt, source SHA, event SHA, and corpus digest before attesting it.
4. `actions/attest` signs the archive's subject digest with GitHub OIDC-backed provenance.
5. The pinned `gh attestation verify` call verifies the signature and enforces the repository, signer workflow, OIDC issuer, SLSA predicate, expected source ref, expected source SHA, and the digest of the local archive.
6. Only after signature and bundle verification does `evaluate-attested-release.sh` re-run the SRQ evaluator on the extracted receipts. It preserves three separate outputs: the CLI verification JSON, the freshly computed qualification report, and an integrity envelope that hashes all three relevant inputs/outputs.

The stand-alone `evaluate-release.sh` remains a convenient contract evaluator for local tests. A release or external qualification consumer must use the attested wrapper, not treat an unsigned receipt plus self-authored digest as authenticated evidence.

## Commands

Pinned CLI installation (supported runner target: Ubuntu Linux x86_64):

```bash
bash benchmarks/srq-v1/install-pinned-gh.sh
```

Attested evaluation is intentionally strict and designed for a main-branch manual workflow run:

```bash
bash benchmarks/srq-v1/evaluate-attested-release.sh \
  --archive /path/to/srq-evidence-bundle.tar.gz \
  --expected-source-sha FULL_40_HEX_COMMIT \
  --expected-source-ref refs/heads/main \
  --expected-workflow-ref Luminous-Dynamics/luminous-platform/.github/workflows/srq-v1.yml@refs/heads/main \
  --expected-run-id RUN_ID \
  --expected-run-attempt RUN_ATTEMPT \
  --output-envelope /path/to/attested-integrity.json \
  --output-report /path/to/recomputed-release-evaluation.json \
  --output-verification /path/to/github-cli-verification.json
```

Values for SHA/ref/run identity must come from the trusted workflow context or another independently trusted release manifest, never from fields copied only from the artifact under review.

## Negative controls and scope

The unit contract tests check that the wrapper passes each trust-policy constraint to the CLI, rejects a wrong CLI version, fails on CLI rejection/empty/malformed output, rejects bad source identities, rejects a changed evidence byte, and rejects unexpected archive inventory. The test double verifies command construction and fail-closed behavior; it is **not** a cryptographic signature test.

The real trust check is the manual main-branch run that signs the archive and successfully executes the pinned CLI against GitHub's attestation API. Tampering with one archive byte, using an unattested archive, or using another repo/workflow/ref must make real verification fail.

## What provenance does not prove

An attestation authenticates an artifact digest and the workflow/source identity under the stated policy. It does not make an incorrect test correct, stop malicious logic in an incorrectly reviewed workflow, independently confirm runtime state, or upgrade the execution tier. Receipt status, qualifying tier, claim ceiling, and corpus completeness are still evaluated separately. The current modeled evidence is still below QEMU/physical required tiers, so the expected evaluation remains **NOT_QUALIFIED (0/22 gates)**.

The attested envelope is a derived convenience artifact, not itself the signed subject. Preserve and independently verify the original signed evidence bundle. Keep issue #93 open until a trusted-main run demonstrates this path and its remaining acceptance tests are independently checked.
