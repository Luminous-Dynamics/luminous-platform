# Independent Security Audit Verifier

## Purpose and status

This is a base-branch-owned verifier for the existing security-audit workflow. It is **not yet an active merge gate**: GitHub only starts a `workflow_run` workflow after the verifier workflow file exists on the repository's default branch. This pull request therefore bootstraps the control; the PR itself must still be reviewed against the existing CI and branch-policy controls before merge.

The verifier publishes the commit status context:

`Security Audit / Independent Verifier`

That string does not become a merge requirement merely because it exists. After the pilot is merged and the first real hosted execution is examined, repository rules must explicitly require the trusted verifier result and must not allow administrators or ordinary workflows to bypass the policy silently.

## What is checked

For the latest run and attempt on an exact open, same-repository PR head, the verifier independently queries GitHub's API and checks:

1. The run ID, attempt, workflow ID/name/path, event type, head branch, exact head SHA, and same-repository origin.
2. The PR is still open and targets the default branch. Fork-originated runs do not qualify for this status.
3. The run is still the latest attempt for that exact subject; stale completions cannot overwrite a newer attempt.
4. The workflow file at the audited commit has the exact reviewed Git blob SHA in the verifier policy. The immutable shared engine commit and its workflow blob are also checked; the platform self-audit checks its local engine blob.
5. The run's unique aggregate-verdict artifact has authoritative run/head metadata, has not expired, and has a SHA-256 digest matching the downloaded ZIP bytes.
6. The ZIP is parsed without extracting it or executing its contents.
The verifier also compares the ZIP's complete non-verdict file set to the verdict's `evidence_files` manifest, recomputes each member's SHA-256, and rejects missing, extra, duplicate, unsafe, or digest-mismatched evidence entries. This prevents an internally inconsistent manifest from passing solely because the outer ZIP digest is correct.
 Unsafe paths, oversized archives, multiple or missing `verdict.json` files, malformed JSON, mismatched run/attempt/subject, missing required lanes, inconsistent finding flags, and `FAIL` or `INCOMPLETE` verdicts all block a passing status.
7. Verifier invariant tests execute on every event. If those tests fail, the verifier attempts to publish a failure status rather than leaving a previously successful context untouched.

A clean `PASS` and a `PASS_WITH_FINDINGS` are distinct. The latter can qualify only when the engine's declared blocking thresholds passed, the verdict records the non-blocking sources consistently, and no failure reason is present. Neither status means the project has undergone a human penetration test or a comprehensive independent security audit.

## Privilege boundary

The workflow runs from the trusted default branch, receives only read access for source, PR and Actions metadata plus permission to publish commit statuses, and does not check out, import, build, run, or execute PR source. It treats the upstream ZIP as untrusted data; only a digest-bound, size-bounded JSON document is parsed. The API token is not sent to the signed artifact-storage URL.

This follows GitHub's warning that privileged `workflow_run` workflows must not execute untrusted PR code or blindly trust artifacts from a preceding workflow. See [GitHub's secure-use guidance](https://docs.github.com/en/actions/reference/security/secure-use) and [workflow event documentation](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows).

## Bootstrap and ongoing policy changes

- Initially, the verifier cannot certify its own introducing PR because this workflow does not execute until it exists on the default branch. Review and merge this bootstrap change using the repository's existing controls; then run the first real audit/verifier cycle and inspect the receipt/status before requiring it.
- The caller workflow and audit engine blob pins are intentionally frozen. A legitimate edit to either control must update the trusted policy in a separate, reviewed default-branch change; a PR must not be able to modify both the audited workflow and its verifier's expected identity.
- After pilot qualification, apply the same base-branch verifier to Mycelix and Symthaea. The policy already inventories their workflow identities and source blobs, but the status should not be represented as active on those repositories until their receiver workflows are installed, merged and exercised.
- For the platform repository, this is privilege-separated from a pull request, but it is not an organization-external trust root: the repository's default branch and ruleset remain authoritative. A stronger cross-repository verifier/GitHub App should eventually attest to the platform self-audit from a separately governed repository.

## Operational semantics

- `requested` / `in_progress`: publish `pending` for the latest eligible exact-head attempt.
- `completed`: publish `success` only after identity, source blob, artifact digest, verdict schema, and required-lane checks all pass.
- Failed, cancelled, incomplete, malformed, stale, or ambiguous evidence must never become a success.
- An audit/verifier outage must not prevent host boot or recovery, but missing or invalid audit evidence must not authorize a merge, privileged mutation, or release qualification.

The artifact digest is taken from GitHub's artifact metadata and independently recomputed over the downloaded archive before parsing. GitHub documents artifact SHA-256 digest support in its [artifact validation guidance](https://docs.github.com/en/actions/tutorials/store-and-share-data).


## Enforcement verification snapshot (2026-10-09)

GitHub's `GET /repos/{owner}/{repo}/branches/main` response reports `protected: false` for this repository, and the repository-level `/rulesets` endpoint returned an empty list. The connected integration's branch-protection detail request returned HTTP 403, and organization-level ruleset policy could not be established from this connection. Thus the available evidence does **not** show an active required-status merge gate on `main`. This is a release blocker for enforcement, not a reason to treat the verifier as passed. A repository administrator must configure and verify the exact `Security Audit / Independent Verifier` status and verifier job as required checks, define controlled bypasses, and confirm organization policy if present.
