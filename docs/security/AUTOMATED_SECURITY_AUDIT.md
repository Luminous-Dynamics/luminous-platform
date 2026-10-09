# Automated Security Audit Architecture

**Status:** initial automation layer; not an independent security audit or a production-security certification.

## Purpose

This repository hosts the reusable workflow used to audit the public Luminous-Dynamics repositories. It separates scanner execution from the repository being scanned and makes the requested source commit explicit. The first implementation covers:

1. GitHub Actions workflow analysis with `zizmor`, using the auditor persona and online audits.
2. RustSec dependency-advisory checks for every *tracked* `Cargo.lock` file when the caller opts into Rust auditing.
3. Machine-readable `cargo-audit --json` output, scanner stderr, lockfile SHA-256 values, exact checked-out commit, caller workflow identity, run identity, tool versions, and an overall pass/fail record.
4. Read-only token permissions, immutable action references, an exact-subject checkout assertion, no repository secrets, and no write-capable token permissions.

The platform self-check runs the workflow audit on pushes to `main`, pull requests, a weekly schedule, and manual dispatch. Callers with Rust dependency graphs set `audit_rust: true`.

## What a result means

- **PASS** means the configured scanner completed for the covered inputs and returned exit status zero.
- **FAIL** means a scanner returned nonzero, the workflow could not establish its subject, or no tracked Rust lockfile was available when Rust auditing was requested.
- **INCOMPLETE** applies at the program level when the repository contains relevant assets outside the scanner's declared scope, when results are missing, or when an independent verifier has not validated the receipt.

A workflow being queued, starting, or finishing successfully is not by itself a security pass. A pass from this initial workflow only covers its declared checks. It does not prove absence of unknown vulnerabilities, correctness of application logic, security of the GitHub organization settings, or resilience against a determined attacker.

## Trust boundaries

- The audited source is checked out at the caller-supplied commit SHA and `git rev-parse HEAD` must match it.
- Third-party workflow actions used by this workflow are pinned to full commit SHAs.
- `contents: read` and `actions: read` are the maximum token permissions used; dependency scanning has only `contents: read`.
- Checkout credentials are not persisted. No audit step receives deployment, publishing, or signing secrets.
- The `zizmor` job produces findings/annotations for review. Its finding state is not converted into a green security certification by this workflow.
- The RustSec job scans committed lockfiles, not every possible dependency-resolution graph. It does **not** yet assert that every application manifest has an associated lockfile, and it does not audit non-Rust ecosystems.

**Important limitation:** a repository-defined workflow is not, by itself, an independent trust anchor. A pull request can alter local workflow definitions. Before treating its result as a merge-authorizing control, configure a ruleset/required workflow that cannot be satisfied by a PR replacing or skipping the intended auditor. Verify the workflow identity, immutable reusable-workflow revision, subject SHA, run attempt, and artifact digest from outside the audited source repository.

## Rollout plan

### Phase 1 — Reliable inventory and evidence

- Run the reusable workflow on Mycelix and Symthaea without changing application behavior.
- Measure committed lockfile coverage and classify scanner failures separately from advisory findings.
- Preserve raw reports and make missing reports explicit; never convert a scanner error into a pass.

### Phase 2 — Dependency and build policy

- Inventory every Cargo manifest and map it to the exact workspace/lockfile that resolves it; treat unexplained gaps as incomplete.
- Add reviewed `cargo-deny` policy for advisories, licensing, banned crates, duplicate versions, and allowed sources. Do not add broad advisory ignores merely to make CI green; exceptions need a narrow scope, owner, rationale, evidence, and expiry.
- Add ecosystem-appropriate checks for JavaScript/TypeScript lockfiles and Nix flakes, with an explicit inventory of their covered roots.

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
- Every tracked `Cargo.lock` discovered by the scanner has one JSON result and a recorded exit code.
- Any nonzero RustSec exit remains a failure; all lockfiles are still attempted so one finding does not hide the remaining inventory.
- Artifact upload succeeds when evidence files exist.
- The final verdict preserves limitations and is independently interpretable; it is not represented as a human penetration test or a complete audit.

## References

- [NIST SP 800-218 — Secure Software Development Framework](https://csrc.nist.gov/pubs/sp/800/218/final)
- [RustSec cargo-audit](https://github.com/rustsec/rustsec/tree/main/cargo-audit)
- [zizmor](https://github.com/zizmorcore/zizmor)
- [SLSA build provenance](https://slsa.dev/spec/v1.0/provenance)
