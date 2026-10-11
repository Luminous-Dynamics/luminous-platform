# Independent Security Audit Verifier

## Purpose and status

This is a base-branch-owned verifier for the existing security-audit workflow. It is **not yet an active merge gate**: GitHub only starts a `workflow_run` workflow after the verifier workflow file exists on the repository's default branch. This pull request therefore bootstraps the control; the PR itself must still be reviewed against the existing CI and branch-policy controls before merge.

The verifier publishes the commit status context:

`Security Audit / Independent Verifier`

That string does not become a merge requirement merely because it exists. After the pilot is merged and the first real hosted execution is examined, repository rules must explicitly require the trusted verifier result and must not allow administrators or ordinary workflows to bypass the policy silently.

## What is checked

For the latest run and attempt on an exact open, same-repository PR head, the verifier independently queries GitHub's API and checks:

1. The run ID, attempt, workflow ID/name/path, event type, head branch, exact head SHA, and same-repository origin. The API `path` must exactly match the canonical base-owned workflow file path; the verdict workflow reference must use that same path and the independently matched PR number.
2. The PR is still open and targets the default branch. Fork-originated runs do not qualify for this status.
3. The run is still the latest attempt for that exact subject; stale completions cannot overwrite a newer attempt.
4. The caller workflow at the audited PR-head commit and at `verdict.workflow_sha` must both match the exact reviewed Git blob SHA in base-owned policy. Additionally, `verdict.workflow_sha` must equal the independently fetched PR `merge_commit_sha`, and GitHub must currently report `mergeable: true`, before the file is checked at that exact test-merge commit. The immutable shared engine commit and its workflow blob are also checked; the platform self-audit checks its local engine blob.
5. The run's unique aggregate-verdict artifact has authoritative run/head metadata, has not expired, and has a SHA-256 digest matching the downloaded ZIP bytes.
6. The ZIP is parsed without extracting it or executing its contents.
The verifier also compares the ZIP's complete non-verdict file set to the verdict's `evidence_files` manifest, recomputes each member's SHA-256, and rejects missing, extra, duplicate, unsafe, or digest-mismatched evidence entries. This prevents an internally inconsistent manifest from passing solely because the outer ZIP digest is correct.
 Unsafe paths, oversized archives, multiple or missing `verdict.json` files, malformed JSON, mismatched run/attempt/subject, missing required lanes, inconsistent finding flags, and `FAIL` or `INCOMPLETE` verdicts all block a passing status.
7. Verifier invariant tests execute on every event. If those tests fail, the verifier attempts to publish a failure status rather than leaving a previously successful context untouched. A missing self-test outcome is also a failure; it cannot default to success.

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




## Pre-merge verifier tests

The unprivileged `.github/workflows/security-audit-verifier-tests.yml` workflow runs the verifier's Python compilation and adversarial unit tests on the exact PR head, with read-only repository access, no secrets, and no write permissions. It is test evidence only: it neither publishes an authorization status nor replaces the default-branch `workflow_run` trust anchor. Hosted results must complete and be inspected; a local unit-test pass is not a hosted CI pass.



## Fail-closed status integrity

A verifier exception can otherwise leave a previous green commit status visible, but unconditionally writing `failure` is also unsafe: another workflow with the same display name could trigger the receiver and poison the status. The verifier now only attempts an exception-path failure update when the authenticated-event fields identify the exact policy-pinned workflow ID, a `pull_request` run, and matching base/head repository IDs. Once the run has been authenticated through the API, failures in its result or evidence still publish a failure. An inability to reach GitHub is reported as incomplete; no software can guarantee a remote status update while the status API itself is unavailable.

The Python test file currently contains 23 test methods. The corresponding hosted runs have not completed yet, so their result remains unverified.

## Enforcement verification snapshot (2026-10-09)

GitHub's `GET /repos/{owner}/{repo}/branches/main` response reports `protected: false` for this repository, and the repository-level `/rulesets` endpoint returned an empty list. The connected integration's branch-protection detail request returned HTTP 403, and organization-level ruleset policy could not be established from this connection. Thus the available evidence does **not** show an active required-status merge gate on `main`. This is a release blocker for enforcement, not a reason to treat the verifier as passed. A repository administrator must configure and verify the exact `Security Audit / Independent Verifier` status and verifier job as required checks, define controlled bypasses, and confirm organization policy if present.





The verifier's exception-path status update is additionally bound to the triggering event's expected workflow ID, `pull_request` event type, same base/head/current repository IDs, and default-branch PR target. Malformed API payloads or unexpected ordinary exceptions are normalized to a verifier failure, while status-write failures themselves are reported without recursively crashing. A failed or untrusted trigger cannot poison the authoritative status merely by sharing the workflow display name.



## Verdict contract and freshness

The external verifier requires the exact luminous.security-audit.verdict.v1 field set; rejects duplicate JSON object keys; requires the caller workflow reference and SHA to be canonical; matches the workflow run URL to GitHub's API record; and rejects timestamps more than five minutes in the future or older than seven days. The policy deliberately requires a new audit before merging a PR whose last audit evidence is stale. The corresponding unit tests are present, but hosted execution has not completed, so behavior remains unqualified until those runs finish.

## Required-check semantics for reruns

GitHub documents that `workflow_run: requested` is not emitted for a re-run; `in_progress` is the early invalidation event for reruns. If a rerun is queued, an older commit status might remain until the trusted receiver can publish its next state. Therefore a production ruleset must require the repository's **producer audit workflow check** as well as the `Security Audit / Independent Verifier` commit status, and must not treat the receiver's own default-branch job as a substitute for a PR-head check. The producer check holds the PR while a new audit attempt is queued/running; the custom status only turns green after independent verification. Test-run status on the PR head before authorizing merge.


## Latest exact-head base-freshness guard — 2026-10-11

- The verifier performs a second authoritative PR API read after validating the artifact and immediately before publishing success. Both reads require the same canonical test-merge SHA.
- Each read also requires an explicit recognized `mergeable_state`; `behind`, `unknown`, missing, and unrecognized states fail closed. This rejects a candidate already known to be stale or unresolved at the time of verification.
- Synchronized verifier blob: `47536b70be613967497764a8b2b43def295d8b03`. Synchronized tests blob: `d405e037c94cb0ba76af03040f84bb52b9238731`. The suite now contains 34 test methods; the new merge-state regression cases reject `behind`, `unknown`, missing, and unrecognized values.
- This does not revoke a green commit status after `main` advances later. The minimum enforcement configuration is: require the producer audit check and `Security Audit / Independent Verifier`, with **Require branches to be up to date before merging** enabled. GitHub documents strict required checks as requiring the topic branch to be up to date, and notes required checks must pass on the latest commit SHA: [ruleset requirements](https://docs.github.com/en/repositories/configuring-branches-and-merges-in-your-repository/managing-rulesets/available-rules-for-rulesets), [required-check troubleshooting](https://docs.github.com/en/pull-requests/how-tos/merge-and-close-pull-requests/troubleshooting-required-status-checks).
- If a merge queue is selected instead, every required workflow needs `merge_group: checks_requested` and a separately qualified merge-group provenance path; this verifier is not currently qualified for merge-group runs. See [GitHub's merge-group event guidance](https://docs.github.com/en/actions/reference/workflows-and-actions/events-that-trigger-workflows).
- Hosted runs for these new changes remain queued or pending. No pass is claimed, and none of these draft PRs is an active merge gate.


## Read-only required-check policy preflight

The Platform PR also adds `.github/scripts/audit_security_required_checks.py`, a read-only preflight for the enforcement step in [issue #24](https://github.com/Luminous-Dynamics/luminous-platform/issues/24). Run it from a trusted administrative workstation after configuring the rules:

```sh
GITHUB_TOKEN="$(gh auth token)" python3 .github/scripts/audit_security_required_checks.py
```

Pass one or more exact repository names to limit scope; without arguments it checks all three policy entries. Use a read-only token with repository Metadata read and Administration read permissions. The command prints one machine-readable JSON receipt and exits 0 only if every selected repository qualifies (1 for a confirmed policy failure, 2 for an incomplete observation). It does not create or edit branch protection/rulesets.

The checker inspects up to 100 GitHub effective active rules per repository's API-reported default branch (and refuses a negative conclusion from a possibly truncated page) plus the classic required-status-check endpoint. It verifies each effective rule's ruleset ID, source type, and source before using it. Required contexts may be distributed across several strict active rulesets because branch rules are additive. The JSON receipt includes the running auditor's source SHA-256, a deterministic digest of the repository policy and required-check constants, the Python implementation/version, and API version so an archived result can be tied to the exact tool and runtime. It requires strict/up-to-date behavior and both the producer's terminal aggregate-verdict job and `Security Audit / Independent Verifier` to be pinned to the GitHub Actions application (`integration_id` / `app_id` 15368). Name-only legacy `contexts[]`, an absent source binding, a different application ID, or an unrecognized repository/default branch cannot qualify. Failure to read the effective active-rules endpoint is `INCOMPLETE`. A 403 from the classic-protection endpoint is tolerated only when the effective-rules endpoint independently proves active strict rules that collectively contain both exact, app-bound contexts, with valid source provenance; otherwise the result is `INCOMPLETE`, never `PASS`. The producer contexts are fixed in reviewed policy: `Shared security audit / Deterministic audit verdict and evidence completeness` for Mycelix and Symthaea, and `Audit platform workflow definitions / Deterministic audit verdict and evidence completeness` for the Platform self-check.

The auditor rejects malformed or oversized effective-rules responses and distinguishes a complete negative finding from a potentially truncated page; all failure receipts retain the same tool/runtime/policy identity fields as complete observations.

The preflight audits only required-check freshness and source binding. It does not certify ruleset bypass actors, administrator enforcement, review policy, merge-queue support, or a real merge attempt; record those separately before declaring the enforcement boundary active. It must run with sufficient read permission and its JSON receipt must be retained as evidence. This design follows GitHub's APIs for [effective branch rules](https://docs.github.com/en/rest/repos/rules?apiVersion=2022-11-28) and [classic required status checks](https://docs.github.com/en/rest/branches/branch-protection?apiVersion=2022-11-28).

The unprivileged verifier-test workflow now compiles all test sources and invokes `.github/scripts/run_security_audit_policy_tests.py`, which loads the exact named suites and requires precisely 34 verifier tests, 32 policy-auditor tests, and 7 runner-self-tests (73 total). It fails if a file is missing/extra, method counts drift, `unittest` reports any skipped/expected-failure/unexpected-success case, or the runner's executed count differs from the declared inventory. The runner itself is covered by `.github/scripts/test_run_security_audit_policy_tests.py`. These are authored checks and inventory contracts; no local or hosted test pass is implied.


Malformed effective-rule provenance and possibly truncated negative observations return `INCOMPLETE`; the auditor will not manufacture a `FAIL` from an observation that may be incomplete. Each ruleset context must bind to GitHub Actions application ID `15368`, and contexts split across multiple active strict rulesets are accepted only when their qualified union contains both required contexts.
