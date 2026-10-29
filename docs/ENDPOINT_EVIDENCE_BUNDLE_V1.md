# Offline CI Evidence Bundle v1

**Status:** verifier and synthetic tests are proposed for review; they do not authenticate a bundle producer or prove an endpoint effect occurred.

## Purpose

The runtime probes for [inherited descriptor hygiene](https://github.com/Luminous-Dynamics/luminous-platform/pull/144) and [pre-opened descriptor authority under Landlock](https://github.com/Luminous-Dynamics/luminous-platform/pull/145) emit small JSON receipts. Their workflows retain the receipt, a run-context file, an evidence manifest, and a SHA-256 checksum list as a time-limited Actions artifact.

The companion [offline verifier](../tools/agentic-endpoint-security-v1/verify_evidence_bundle.py) lets another researcher check the downloaded bundle without trusting a screenshot, excerpted CI log, or the artifact's self-declared pass status.

## Verification inputs

The caller must provide two values obtained independently from the GitHub run/PR page:

- \`--expected-checkout-sha\`: exact commit intended to be tested.
- \`--expected-run-id\`: Actions run ID associated with that execution.

Example:

\`\`\`sh
python3 tools/agentic-endpoint-security-v1/verify_evidence_bundle.py \\
  --bundle-dir ./landlock-preopened-fd-evidence \\
  --expected-checkout-sha 0123456789abcdef0123456789abcdef01234567 \\
  --expected-run-id 40900000000
\`\`\`

Use the real SHA and run ID for the downloaded artifact; the values above are examples, not actual evidence identifiers.

## Checks performed

1. Verifies that \`SHA256SUMS\` covers exactly the receipt, run-context and evidence-manifest files, with no duplicate or path-like filenames, and that their bytes match the listed digests.
2. Checks the evidence-manifest schema and requires it to disclose that the manifest is not signed.
3. Compares the manifest's checkout SHA and run ID to the caller-supplied expected values.
4. Checks that the run-context agrees with the manifest about source SHA, run ID, attempt, event, workflow and individual CI step outcomes.
5. Verifies that the manifest's receipt digest matches the raw receipt bytes and that the manifest's receipt schema/status match the JSON receipt.
6. Applies the per-receipt contract: exact scope, required positive/negative controls, kernel identity, and Landlock ABI where applicable.
7. Recomputes whether the bundle can qualify for the declared micro-test from receipt validity and the outcomes of formatting, Rust tests, the runtime experiment, and Clippy.

The verifier has two distinct non-success results:

- Exit code 1: internally consistent, but \`not_qualified\` (for example, the receipt was never emitted or a required CI step failed).
- Exit code 2: invalid or inconsistent bundle, source/run mismatch, checksum error, unsupported schema or claim escalation.
- Exit code 0: candidate pass for the named micro-test only.

## Trust and claim limits

- A checksum detects accidental or post-download modification only when the expected checksum source is trusted. Since \`SHA256SUMS\` is stored in the same artifact, it is not an independent signature.
- The manifest's checkout SHA and run ID must be compared against external values. Self-consistency alone cannot establish who published an artifact.
- The verifier does not prove GitHub's workflow identity, artifact authenticity, host-kernel integrity, Landlock correctness, source observation truth, or general sandbox safety.
- A \`candidate_pass_for_declared_microtest\` is scoped to the tested descriptor/exec boundary on the named runner. It is not an endpoint-security or SOTA superiority claim.
- The artifact retention period is finite. Archive evidence separately for any long-term publication, and publish the source commit plus checksums alongside any result.

## Test strategy

\`test_verify_evidence_bundle.py\` uses synthetic receipts for both runtime probes and mutation tests for wrong source SHA/run ID, changed bytes, missing checksum entries, a failed control, a failed CI step, false qualification labels, outcome mismatch, unsupported receipt schemas, non-emitted receipts, and attempted claim inflation. These tests validate the verifier's contract; they do not run the Landlock experiment.


## First runtime artifact examples

As of the current research snapshot (2026-10-29), two exact-head Linux micro-test runs completed all of their declared checks:

- [Landlock descriptor-ordering run 40951705435](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/40951705435), receipt SHA-bound to `2d7402a6aeef5fa3aca24de019a4f315213bfe73`, Landlock ABI 8, runner kernel release `7.0.0-1017-azure`.
- [close_range descriptor-inheritance run 40951695669](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/40951695669), receipt SHA-bound to `525fcce3f3ec875fcef2cb26f09ab8f8dc69f49c`, runner kernel release `7.0.0-1017-azure`.

The respective artifact bundles are [Landlock evidence bundle](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/40951705435/artifacts/12931677316) and [FD-inheritance evidence bundle](https://github.com/Luminous-Dynamics/luminous-platform/actions/runs/40951695669/artifacts/12931013405). Both archives were downloaded during authoring; their listed SHA-256 digests matched all included files, and receipt/run-context/manifest values agreed. The receipts describe only the declared micro-tests. Those checksums are self-contained consistency checks, not signed publisher authentication, and the artifacts have finite retention.


## Cryptographic provenance for successful main-branch evidence

The two runtime workflow drafts, [#144](https://github.com/Luminous-Dynamics/luminous-platform/pull/144) and [#145](https://github.com/Luminous-Dynamics/luminous-platform/pull/145), now add a separate attestation job with these rules:

- It runs only for a successful push to refs/heads/main, after the runtime test job succeeds. Pull-request jobs do **not** receive id-token: write or attestations: write.
- The attestation job downloads that exact run-and-attempt's evidence bundle and rechecks checksums, source SHA, workflow SHA, run ID/attempt, step outcomes, runtime receipt fields, and claim ceiling before signing.
- The subject is evidence-manifest.json, which binds the receipt digest, run identity, step outcomes, kernel identity and explicit claim ceiling. The job uses GitHub's keyless OIDC/Sigstore action actions/attest pinned to commit 1e69f48acb82d1966a394da916b4c1698aa569d6; the download action is pinned to 634f93cb2916e3fdff6788551b99b062d0335ce0.
- The Sigstore attestation bundle is uploaded as a separate 14-day artifact (landlock-preopened-fd-attestation-... or fd-hygiene-attestation-...). Keeping it separate avoids a self-referential digest: the signed subject is the evidence manifest, whose own digest inventory covers the receipt and run context.

These are draft workflow changes; signing is **not yet demonstrated on main**. The existing successful PR-run artifacts described above are checksummed but not yet backed by a GitHub artifact attestation. A successful PR check cannot mint the main-branch attestation because the privileged job is explicitly gated to a trusted main push.

### Online verification

Download the unsigned evidence artifact and the separate attestation bundle for the same run. First verify the manifest's Sigstore provenance against the repository and exact workflow identity. Example for the Landlock workflow:

    gh attestation verify evidence/evidence-manifest.json \
      --repo Luminous-Dynamics/luminous-platform \
      --signer-workflow github.com/Luminous-Dynamics/luminous-platform/.github/workflows/landlock-preopened-fd-probe.yml \
      --source-ref refs/heads/main \
      --source-digest "$EXPECTED_CHECKOUT_SHA"

For the inherited-FD workflow, use github.com/Luminous-Dynamics/luminous-platform/.github/workflows/fd-hygiene-probe.yml as the signer workflow. Set EXPECTED_CHECKOUT_SHA to the exact SHA expected from the workflow run, obtained independently from GitHub—not from the bundle itself.

Then run the offline bundle consistency verifier from this repository. Its candidate_pass_for_declared_microtest still qualifies only the named micro-test; the signature verifies provenance of the manifest bytes, not the truth of the underlying kernel observations.

### Offline verification

On an online staging machine, download the attestation bundle from the separate attestation artifact and export the Sigstore trusted root:

    gh attestation trusted-root > trusted_root.jsonl

Move the evidence manifest, its original evidence files, the .sigstore.json bundle, and trusted_root.jsonl into the offline verification environment. Verify the attestation bundle before using the evidence:

    gh attestation verify evidence/evidence-manifest.json \
      --repo Luminous-Dynamics/luminous-platform \
      --bundle signed/evidence-manifest.sigstore.json \
      --custom-trusted-root trusted_root.jsonl \
      --signer-workflow github.com/Luminous-Dynamics/luminous-platform/.github/workflows/landlock-preopened-fd-probe.yml \
      --source-ref refs/heads/main \
      --source-digest "$EXPECTED_CHECKOUT_SHA"

Use the FD workflow path instead for an FD-inheritance manifest. For long-lived or higher-assurance archival, also retain the verified source repository ID and trusted-root version; offline certificate-name matching alone cannot detect a later repository rename or name reuse. See [GitHub's offline attestation verification instructions](https://docs.github.com/en/actions/how-tos/secure-your-work/use-artifact-attestations/verify-attestations-offline) and [gh attestation verify](https://cli.github.com/manual/gh_attestation_verify).

**Trust ceiling:** an attestation establishes who/which workflow signed the manifest and binds its digest under the accepted trust policy. It does not make the workflow's own predicate magically truthful, prove the host kernel uncompromised, broaden the runtime test, or establish SOTA superiority.

## One-command verification of signed evidence

The new [signed evidence verifier](../tools/agentic-endpoint-security-v1/verify_attested_evidence_bundle.py) composes the checks above with GitHub CLI's Sigstore verification. It requires the evidence bundle, the separate attestation bundle, the Sigstore trusted root, the independently expected checkout SHA, the Actions run ID, and the expected GitHub repository ID. It chooses the expected workflow identity from the probe type and requires the main-branch source ref.

First, download and unpack both the evidence artifact and the matching attestation artifact from the same successful main-push run. On an online staging machine, capture the trusted root and repository ID:

```sh
gh attestation trusted-root > trusted_root.jsonl
gh api repos/Luminous-Dynamics/luminous-platform --jq .id > repository_id.txt
```

Then use the signed verifier in the trusted repository checkout (or copy the verifier and its sibling offline verifier to the verification host):

```sh
python3 tools/agentic-endpoint-security-v1/verify_attested_evidence_bundle.py \
  --evidence-dir ./landlock-evidence \
  --attestation-dir ./landlock-attestation \
  --trusted-root ./trusted_root.jsonl \
  --expected-checkout-sha "$EXPECTED_CHECKOUT_SHA" \
  --expected-run-id "$EXPECTED_RUN_ID" \
  --expected-repo-id "$(cat repository_id.txt)" \
  --probe landlock
```

Use --probe fd for the inherited-descriptor workflow. The expected source SHA and run ID must be obtained independently from the run/commit page; the expected repository ID must be obtained independently from the GitHub API. Do not copy those expectations out of the evidence bundle.

The verifier checks the evidence bundle's exact inventory and checksums, binds source SHA and run ID, enforces the probe-specific claim ceiling, runs gh attestation verify with the pinned expected signer workflow, source ref and source digest, and then verifies the attested manifest digest and the certificate's signed sourceRepositoryIdentifier field against the repository ID supplied independently by the verifier. This explicit post-check is necessary for offline bundles: the CLI's built-in repository-name check alone does not detect repository rename or name reuse. It returns exit 0 only for a signed, provenance-verified candidate pass for the named micro-test; invalid evidence returns exit 2. Internally consistent but nonqualified evidence is rejected before invoking the signature verifier.

The test suite uses synthetic attestation JSON and a mocked GitHub CLI process; it does not generate signatures or replace a real Sigstore verification. The real verifier always invokes GitHub CLI on the supplied bundle and trusted root.