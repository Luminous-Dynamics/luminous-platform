# OpsBench Sealed Corpus and Contamination Controls v0.1

**Status:** sealed-corpus protocol prototype with synthetic example fixtures. No real confirmatory corpus, deployed IAM isolation, private salt custody, network control or signed access-log provenance is qualified.

## 1. Why this exists

A private-variant digest in a preregistration is necessary but insufficient. It does not prove the underlying scenarios were private, that the secret salt remained outside the agent's reach, that the answer key was isolated, or that the confirmatory split avoided close relatives of development tasks.

The current research signal is strong:
- The NeurIPS 2025 Agentic Benchmark Checklist paper identifies task-setup and reward-design flaws that can severely distort reported agent performance; its checklist reduced CVE-Bench overestimation by 33%.
- An October 2026 EMNLP Findings paper on search-time contamination reports up to 4% performance inflation on the deep-research benchmark evaluations it studied, due to retrieval of benchmark metadata, context or answers.
- Artificial Analysis Intelligence Index v5, announced October 27, 2026, reports that 60% of its index now uses private datasets or held-out answers.

These findings do not prove a specific benchmark is contaminated. They demonstrate why answer-key secrecy, controlled egress, hidden variants and pre-trial commitments are now essential methodology.

## 2. Paradigm: reproducible generation, sealed instances

Publish the generator protocol, taxonomy, preregistration, commitment and split policy before scored runs. Keep the concrete confirmatory variant instances, answer keys, oracle implementation and salt inside a separate evaluator trust domain. Reveal the salt and public-safe corpus after study closure so independent reviewers can verify the pre-trial commitment.

The lifecycle is:

1. **Preregister:** pin generator, evaluator, source, environment, tool surface, policy, scenario split and target count.
2. **Generate:** create evaluator-owned scenario and answer-key artifacts from versioned recipes.
3. **Split:** assign whole fault-family and topology groups to development, screening or confirmatory; never split near-identical variants across those groups.
4. **Commit:** compute the domain-separated salted commitment over the exact manifest bytes. Publish the commitment and preregistration digest before the first scored trial.
5. **Seal:** keep manifest, answer keys, oracle code and salt inaccessible to SUT principals. Only expose task prompts, permitted telemetry, and approved service endpoints.
6. **Audit:** collect filesystem, network, tool-call, process, secret and artifact-store coverage for each started episode. Canary hits or successful forbidden access are contamination failures; incomplete coverage is indeterminate.
7. **Close and reveal:** freeze the trial set and results, then disclose salt and permissible artifacts. Recompute the commitment and preserve the exact verifier report.

## 3. Commitment construction

v0.1 uses SHA-256 with domain separation and a high-entropy secret salt:

    commitment = SHA256(
        UTF8("luminous-opsbench-sealed-corpus-v0.1") || 0x00 ||
        salt_at_least_32_bytes || 0x00 ||
        exact_raw_manifest_bytes
    )

The public registration record binds:
- study ID and preregistration digest;
- exact raw manifest digest;
- commitment scheme/domain and commitment digest;
- salt digest (not salt);
- preregistration, registration and first-trial timestamps;
- post-study reveal timestamp when applicable.

Do not canonicalize/re-serialize the manifest before hashing. The checker binds exact raw bytes. A one-byte change invalidates the commitment. A commitment mismatch invalidates the study; do not silently replace an artifact or recompute after results are known.

The repository's salt.example.txt is deliberately public and insecure. It is only a test fixture. A real confirmatory salt must be generated from a cryptographically secure source, kept in evaluator custody, excluded from git, agent mounts, CI logs and shared object stores, and disclosed only after closure.

## 4. Split and contamination rules

The manifest gives each item:
- scenario ID and variant ID;
- fault-family group and topology group;
- prompt, answer-key, oracle, environment, policy and tool-surface digests;
- license/source identifier;
- split assignment.

v0.1 rejects:
- duplicate scenario/variant keys;
- one scenario ID assigned to multiple splits;
- a fault-family group reused across splits;
- a topology group reused across splits.

The confirmatory manifest must contain at least 500 distinct scenarios. The target is a starting floor, not proof of broad population representativeness. The manifest is a commitment to a frozen evaluation corpus, not permission to use public tasks as the blind test set.

For generation and calibration:
- use development items for engineering;
- use screening items for prompt/model/harness iteration;
- freeze confirmatory items before any confirmatory result is inspected;
- review task difficulty and item discrimination across arms, including saturation and impossible-task checks;
- create new confirmatory versions only for future study IDs, never by replacing failed or inconvenient items in the current study.

## 5. Evaluator access policy

The SUT should see only:
- the exact task prompt;
- declared telemetry and task APIs;
- the allowed mock/service endpoints;
- its isolated workspace.

The SUT must not see:
- answer keys or expected outcome tables;
- oracle/evaluator implementation;
- generator source and private manifest;
- commitment salt or secret material;
- evaluator logs, scoring output or benchmark repository;
- unrestricted network/search egress.

The example policy defines allowed resource classes and allowed actions separately. For example, a task prompt may be readable but not executable or overwritten. Network egress is restricted to an exact endpoint allowlist.

The access audit is per started run/arm/scenario/variant/seed. Required channels are filesystem, network, tool calls, process, secrets and artifact store. A successful forbidden read/mount/execute or unapproved egress is a failure. A denied attempt is retained as a blocked-attempt observation. An unknown event or missing coverage makes the result indeterminate. Canary hits fail and require adjudication; the canary content itself is not printed into the public report.

## 6. Run the structural checker

The checker verifies:
- raw manifest bytes and salted commitment;
- exact preregistration bytes and study binding;
- timestamp ordering;
- distinct split assignments for scenario, family and topology groups;
- confirmatory scenario-count floor;
- exact SUT resource/action policy;
- observed forbidden access, unapproved egress, blocked attempts, canary hits and missing audit coverage.

Example:

    python3 tools/opsbench-sealed-corpus-v0.1/check_sealed_corpus.py \
      --manifest benchmarks/opsbench-sealed-corpus-v0.1/manifest.example.json \
      --registration benchmarks/opsbench-sealed-corpus-v0.1/registration.example.json \
      --salt-file /secure/evaluator/sealed-corpus.salt \
      --preregistration benchmarks/opsbench-common-core-v0.1/preregistration.example.json \
      --access-policy benchmarks/opsbench-sealed-corpus-v0.1/access-policy.example.json \
      --access-audit /secure/evaluator/run-access-audit.json

For the checked-in example only, replace the secret-salt path with salt.example.txt. Never use that fixture for a live study.

## 7. What the checker does not prove

The checker is a structural and integrity tool. It does not:
- authenticate who created or published the registration, manifest or audit events;
- prove that a salt was secret or that evaluator storage/IAM/network policy was actually enforced;
- prove audit collection observed every possible access path;
- scan external search indexes or verify every near-duplicate across the public internet;
- prove the oracle's correctness or evaluator independence.

Those require the producer-authenticated evidence profile (#178), independent access-control configuration, negative-control penetration tests, signed access logs, and a post-study reviewer. A hash commitment proves that the same bytes are being checked; it does not prove the bytes were honestly generated or privately held.

The synthetic example is example_only and has one confirmatory scenario. It intentionally does not satisfy the live 500-scenario threshold. It is not a benchmark score, safety certification or evidence that a production environment is sealed.

## 8. Promotion gates

Before a confirmatory OpsBench study:
1. Register immutable preregistration and salted corpus commitment before first trial.
2. Demonstrate by negative controls that SUT identities cannot read answer keys, evaluator code, private manifests or salts.
3. Demonstrate egress isolation and complete six-channel access coverage.
4. Run split-leakage and near-duplicate scans across prompt, topology, fault family, telemetry and answer-key-derived text.
5. Ensure canary probes do not leak answers and any hit is fully adjudicated.
6. Keep all started trials, access records and failed/indeterminate episodes in the denominator.
7. After closure, reveal and reproduce the exact committed manifest bytes and publish the access-contamination report.

No superiority claim is eligible until the study also satisfies #176 preregistration, #172 orthogonal safety assessment, #178 producer authentication and #179 paired scenario-cluster analysis.
