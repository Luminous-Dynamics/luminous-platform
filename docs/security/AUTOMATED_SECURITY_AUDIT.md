# Automated Security Audit Architecture

**Status:** initial automation layer; not an independent security audit or a production-security certification.

## Purpose

This repository hosts the reusable workflow used to audit the public Luminous-Dynamics repositories. It separates scanner execution from the repository being scanned and makes the requested source commit explicit. The first implementation covers:

1. GitHub Actions workflow analysis with `zizmor` v1.30.1, using the auditor persona and online audits; both the action commit and scanner release are pinned. The reusable engine runs `actionlint` v1.7.12 plus ShellCheck v0.11.0 over every committed workflow YAML file in each audited repository; both release archives are SHA-256 verified before execution. An independent `Security Audit Workflow Syntax` workflow runs actionlint v1.7.12 and ShellCheck v0.11.0 over the repository's workflow YAML and embedded shell scripts. Both binaries are downloaded at pinned versions and verified against SHA-256 checksums; the workflow scans every tracked `.yml`/`.yaml` file under `.github/workflows`, including its own definition, so syntax checks do not depend solely on the reusable audit job starting successfully.
2. RustSec dependency-advisory checks for every *tracked* `Cargo.lock` file when the caller opts into Rust auditing, plus deterministic coverage mapping from each tracked `Cargo.toml` to its nearest workspace and tracked lockfile.
3. Machine-readable `cargo-audit --json` output, scanner stderr, a JSON manifest-coverage report, lockfile-inventory and coverage-report SHA-256 values, and per-lockfile SHA-256 digests for both raw JSON and stderr output. Each result records the lockfile digest, exit code, and report-parse status; the scanner output must parse as the expected JSON structure and satisfy internal consistency invariants (for example, the vulnerability count, `found` flag, and finding-list length must agree) or the audit fails. A nonzero vulnerability count remains a failure even if the scanner unexpectedly exits zero; informational warnings are recorded distinctly from a clean zero-finding result. The exact RustSec advisory-database commit, exact checked-out commit, caller workflow identity, run identity, tool versions, and overall pass/fail status are also recorded. Workflow-analysis output is preserved as a separate artifact from RustSec output.
4. Optional npm security auditing for tracked `package-lock.json` and `npm-shrinkwrap.json` files, requiring a sibling tracked `package.json`, using Node.js 24 and `npm audit --audit-level=high --package-lock-only`. Each lockfile/manifest pair and raw report/stderr is hashed and recorded; the report must use supported npm audit report version 2, parse as the expected object structure, and have severity counts that match the report's vulnerability objects, or the audit fails. The blocking policy is high/critical: lower-severity findings are retained as `PASS_BELOW_BLOCKING_THRESHOLD_WITH_FINDINGS`, never described as a clean scan. If both lockfiles exist in one package root, the package-lock report explicitly records `SKIPPED_NOT_AUTHORITATIVE` and points to the shrinkwrap SHA, because npm gives `npm-shrinkwrap.json` precedence. The job disables persisted checkout credentials, uses isolated npm user/global configuration and cache, pins the public npm registry, enforces TLS verification, explicitly enables the audit and online mode, includes production/development/optional/peer dependency classes even if project config requests omissions, and disables scripts. It does not install dependencies or execute project lifecycle scripts.
5. Read-only token permissions, immutable action references, an exact-subject checkout assertion, no repository secrets, and no write-capable token permissions. GitHub Actions used by the engine are pinned to full SHAs for Node 24-capable releases.

The platform self-check runs the workflow audit on pushes to `main`, pull requests, a weekly schedule, and manual dispatch. Callers with Rust dependency graphs set `audit_rust: true`; repositories with tracked npm lockfiles should set `audit_node: true`. Because `npm audit` sends package names and versions to the configured registry for advisory matching, enabling `audit_node: true` is also an explicit data-disclosure decision. Do not enable this lane for private/proprietary dependency metadata until the registry disclosure is approved; this version intentionally pins the public npm registry.

The npm inventory includes tracked lockfiles in the root repository and in verified direct submodules. Before fetching a submodule, the job checks an exact `mycelix-health` name/path/URL allow-list; it then verifies the checkout SHA against the root repository's gitlink SHA and detects nested gitlinks. Unknown/unverified/nested submodules fail the overall lane rather than being treated as covered. The manifest-tracking check runs in the correct repository for both root and submodule files. Tracked Cargo/npm lockfiles that are missing, non-regular files, or symlinks are rejected before invoking the scanner, and the receipt hashes the link target reference instead of following it. The npm audit command pins the public npm registry, explicitly uses the Ubuntu system CA bundle, enables TLS verification, clears npm proxy settings on the command line, and uses isolated user/global configuration and cache locations. This matters because npm loads project `.npmrc` as well as user/global configuration; command-line flags take precedence over those files.

The self-check and consumer entrypoints include `github.run_id` in their concurrency groups. That is deliberate: a newer push may supersede ordinary build CI, but it should not replace a pending audit receipt for an older exact commit before the audit can start. Every audited subject remains separately identifiable by its SHA and run ID/attempt.

The reusable workflow now has a final `verdict` job that runs even when one or more scanner jobs fail. It downloads the expected lane artifacts, checks actual job outcomes plus receipt uniqueness, subject SHA, declared engine SHA, engine-identity status, and each lane's required PASS fields. Before passing, it recomputes the Zizmor/actionlint report digests, the RustSec coverage-manifest digest, and the sorted per-lockfile/per-package JSON and stderr digest inventories against the bytes actually downloaded. It writes `luminous.security-audit.verdict.v1` as `verdict.json`, including lane states, failure reasons, run/attempt identity, the GitHub workflow ref and SHA, a direct run URL, a UTC generation timestamp, the configured 30-day retention for the aggregate artifact, and SHA-256 digests for the evidence files it received. The retention field records workflow configuration only; repository/organization retention controls may differ. Missing, ambiguous, skipped-but-required, or mismatched evidence blocks the verdict. `PASS_WITH_FINDINGS` is distinct from `PASS`: it means only that the configured npm high/critical blocking threshold passed while lower-severity findings remain recorded.

This aggregation is deliberately **producer-side**: because it runs in the same workflow and repository context as the scanned subject, it improves completeness and operator clarity but does not establish independent trust in the workflow source, the runner, or the artifact-service metadata. Merge authorization still requires an external verifier to validate the GitHub run, exact reusable-workflow reference, subject and artifact provenance. The JSON contract is specified in [`audit-verdict-v1.schema.json`](audit-verdict-v1.schema.json) using JSON Schema 2020-12; any external verifier should validate both the structure and the cross-field policy (especially the distinction between `PASS`, `PASS_WITH_FINDINGS`, `FAIL`, and `INCOMPLETE`).

## What a result means

- **PASS** means every requested lane completed, its receipt matched the exact subject/engine identities and required result fields, and the aggregate verifier found no blocking failure within the declared scope.
- **PASS_WITH_FINDINGS** means configured blocking thresholds passed but lower-severity npm findings remain. It is not a clean dependency scan.
- **FAIL** means a scanner reported a blocking finding or nonzero status, a successfully exited scanner produced malformed/inconsistent output, a Cargo manifest has no mapped tracked workspace lockfile, an npm lockfile has no tracked sibling `package.json` (with a structured `scanner_invoked=false` report and stderr evidence), a submodule is missing/unverified/not allow-listed or a nested submodule remains unaudited, or a receipt/required lane explicitly failed.
- **INCOMPLETE** means a required job did not finish, a receipt or report is missing/ambiguous/mismatched, or the run cannot establish its subject or engine identity. Program-level assurance is also incomplete while relevant assets remain outside the declared scanner scope or before an external verifier validates the run and workflow provenance.

A workflow being queued, starting, or finishing successfully is not by itself a security pass. A pass from this initial workflow only covers its declared checks. It does not prove absence of unknown vulnerabilities, correctness of application logic, security of the GitHub organization settings, or resilience against a determined attacker.

## Availability versus authorization

The platform's host-availability contract and its security decision contract must remain separate:

- **Availability may fail open only for non-authoritative/ancillary services.** An audit service outage must not prevent a host from booting or make essential recovery depend on the remote audit plane.
- **Security authorization fails closed.** A missing, stale, malformed, untrusted, or incomplete audit receipt cannot authorize a merge, a privileged mutation, or a claim that a release is qualified.
- A model-produced risk score or a Mycelix trust/reputation value may prioritize findings, but cannot override a deterministic block, a failed scanner, or absent evidence.
- Security-critical policy enforcement must work when Symthaea, Mycelix, network connectivity, or the audit service is unavailable. Those systems can improve analysis and coordination; they must not become a single point of authorization failure.


## Trust boundaries

- The audited source is checked out at the caller-supplied commit SHA and `git rev-parse HEAD` must match it.
- Third-party workflow actions used by this workflow are pinned to full commit SHAs, the core checkout/artifact/Node actions use Node 24-capable releases, and the `zizmor` scanner version is pinned to `v1.30.1` instead of resolving `latest` on each run.
- `contents: read` and `actions: read` are the maximum token permissions used; dependency scanning has only `contents: read`.
- Checkout credentials are not persisted. No audit step receives deployment, publishing, or signing secrets.
- The `zizmor` job produces findings/annotations for review. Its finding state is not converted into a green security certification by this workflow.
- Receipts carry `declared_audit_engine_sha` separately from `caller_workflow_sha`. The declared engine SHA is caller-supplied metadata, not self-authenticating proof; an external verifier must compare it with the immutable `uses: Luminous-Dynamics/luminous-platform/.github/workflows/security-audit.yml@<sha>` reference in the exact audited caller commit.
- Each audit lane also runs a deterministic consistency check against the caller workflow file at the exact subject SHA: external callers must have one full-SHA shared-workflow reference and one matching `audit_engine_sha` input; the platform self-check must use its known local workflow path and match its declared event SHA. This catches mismatch/ambiguity, but is not independent authorization because a PR can change its own caller workflow.
- The final `verdict` job downloads the workflow/RustSec/npm receipts and emits a versioned aggregate verdict plus an evidence-file SHA-256 inventory. It checks lane job outcomes and required receipt fields, but it is still produced by this same workflow run; it must not be treated as an independent trust anchor or as proof that GitHub's reported run identity is authentic.
- The RustSec job scans committed lockfiles against one cloned and recorded RustSec advisory-database commit per run, and maps tracked `Cargo.toml` files to the nearest workspace lockfile. Direct submodules are initialized only when their HTTPS URL is allow-listed to the same GitHub owner; their checked-out `HEAD` must match the exact parent gitlink SHA, and their tracked lockfiles/manifests are included. Unknown URLs, missing/mismatched submodule checkouts, untracked manifest-lock mappings, and nested submodules force the coverage gate to fail. This remains a conservative static inventory rather than Cargo's complete resolved package model. The optional npm job currently scans the root repository tree only; it does not recurse into submodules and cannot prove all dynamic/build-time dependency graphs are represented by npm lockfiles.

**Important limitation:** a repository-defined workflow is not, by itself, an independent trust anchor. A pull request can alter local workflow definitions. Before treating its result as a merge-authorizing control, configure a ruleset/required workflow that cannot be satisfied by a PR replacing or skipping the intended auditor. Verify the workflow identity, immutable reusable-workflow revision, subject SHA, run attempt, and artifact digest from outside the audited source repository.

## Rollout plan

### Phase 1 — Reliable inventory and evidence

- Run the reusable workflow on Mycelix and Symthaea without changing application behavior.
- Measure committed lockfile coverage and classify scanner failures separately from advisory findings.
- Preserve raw reports and make missing reports explicit; never convert a scanner error into a pass.

### Phase 2 — Dependency and build policy

- Inventory every Cargo manifest and map it to the exact workspace/lockfile that resolves it; treat unexplained gaps as incomplete.
- Add reviewed `cargo-deny` policy for advisories, licensing, banned crates, duplicate versions, and allowed sources. Do not add broad advisory ignores merely to make CI green; exceptions need a narrow scope, owner, rationale, evidence, and expiry.
- The reusable engine now has an optional `audit_node: true` path for npm lockfiles. Extend it to explicitly inventoried submodules/workspaces and add Nix flake input auditing with equally explicit scope and evidence.

### Phase 3 — Source and configuration analysis

- Add maintained Rust/TypeScript SAST (for example, CodeQL where available), secret scanning, IaC/Nix analysis, and targeted fuzz/property tests.
- Use SARIF where it improves triage, but treat uploading results as a separate capability requiring narrowly scoped permissions.
- Review new high-confidence findings with a fixed response SLA; introduce blocking thresholds only after the baseline is measured and triage ownership exists.

### Phase 4 — Adversarial assurance

- Exercise authentication, authorization, capability revocation, replay, freshness, non-equivocation, signature/encoding boundaries, and fail-closed behavior with negative tests and mutation testing.
- For Mycelix, prioritize Holochain zome validation and multi-agent/DHT behavior; unit tests alone are insufficient for those trust boundaries.
- For Symthaea and platform components, test untrusted input handling, process boundaries, file/host mutation authority, update/rollback behavior, and privilege separation.
- Simulate malicious pull requests, action/ref substitution, missing or stale evidence, changed lockfiles, and bypass attempts against required checks.

### Phase 5 — Cross-repository evidence plane

- Define a versioned receipt schema binding repository, exact subject SHA, workflow identity/revision, tool versions, inputs and lockfile digests, run ID/attempt, raw-result digest, verdict, limitations, and expiry.
- Have a verifier outside the repository being audited independently check the GitHub run, trusted workflow reference, subject identity, and report integrity. The verifier must not accept a producer-authored `PASS` string as proof.
- Let Mycelix carry signed, append-only provenance and let Symthaea rank/prioritize findings or propose fixes. Keep the authoritative decision deterministic and policy-driven; neither a trust score nor a model-generated explanation may override a hard security failure.
- Generate SPDX/CycloneDX SBOMs and build provenance for release artifacts, then cryptographically bind attestations to the artifact digest. Use an independently bootstrapped trust root rather than treating the same repository's own key as sufficient.

## Initial success criteria

The first rollout is complete only when all of the following are evidenced:

- The audited commit SHA equals the requested subject SHA.
- The workflow-analysis job ran against the committed `.github/` tree.
- Every tracked `Cargo.lock` discovered by the scanner has one JSON result and a recorded exit code, all evaluated against one recorded RustSec advisory-database commit.
- Every tracked `Cargo.toml` in the root repository and each verified direct submodule maps to its expected tracked workspace lockfile; no nested or unverified submodule remains outside the scanned tree.
- When `audit_node: true`, every tracked root-tree npm lockfile has a tracked sibling `package.json`, a raw JSON report and stderr record, input digests, and a blocking high-severity audit exit status. A package-lock shadowed by a sibling shrinkwrap must be explicitly recorded as non-authoritative, while the shrinkwrap is the one scanned.
- The workflow, RustSec, npm (when enabled), and aggregate verdict jobs preserve separate evidence artifacts; the final verdict includes the exact requested subject/engine identities and SHA-256 digests for every downloaded evidence file. Missing or ambiguous required receipts cannot result in PASS.
- Any nonzero RustSec exit or malformed/unexpected audit JSON remains a failure; all lockfiles are still attempted so one finding does not hide the remaining inventory. npm reports are subject to the same parseability requirement.
- Artifact upload succeeds when evidence files exist.
- The final verdict preserves limitations and is independently interpretable; it is not represented as a human penetration test or a complete audit.

## References

- [NIST SP 800-218 — Secure Software Development Framework](https://csrc.nist.gov/pubs/sp/800/218/final)
- [RustSec cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [zizmor](https://github.com/zizmorcore/zizmor)
- [SLSA build provenance](https://slsa.dev/spec/v1.0/provenance)
