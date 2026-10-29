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
