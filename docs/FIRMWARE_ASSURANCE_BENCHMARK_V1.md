# Firmware Assurance Benchmark v1

**State:** proposed design and benchmark contract; not implementation, certification, or device qualification evidence.  
**Parent:** [Luminous Platform #23 — global vendor adapters and firmware lifecycle](https://github.com/Luminous-Dynamics/luminous-platform/issues/23)  
**Related:** [Nixward #17 — UKI signature and firmware trust authorization](https://github.com/Luminous-Dynamics/nixward/issues/17), [Nixward #30 — timestamped revocation](https://github.com/Luminous-Dynamics/nixward/issues/30), [Symthaea #7430 — Secure Boot state and consent](https://github.com/Luminous-Dynamics/symthaea/issues/7430), [Symthaea #4430 — typed device resources and firmware-operation boundaries](https://github.com/Luminous-Dynamics/symthaea/issues/4430).

## 1. Thesis: assurance is a graph of bounded claims, not a security score

Do not invent a single scalar "firmware security score." It would conceal which facts were measured, which were assumed, which evidence is stale, and which checks never ran.

Represent an assurance decision as a versioned graph of **typed claims**, **evidence**, **verifier activities**, and **explicit dependencies**. Each claim refers to one exact subject and scope. Each derived claim identifies the verifier and policy revision that produced it. Missing or ambiguous prerequisites remain `unknown`; they are not promoted to `pass` by confidence, consensus, signature presence, or a successful command exit.

The graph enables a consequential decision to answer: *what exactly was established, from which evidence, under which policy, by which verifier, for which device and artifact, and what remains unresolved?*

This design composes existing standards instead of creating a new firmware signature or update protocol.

## 2. Scope and authority boundaries

The first milestone is **read-only firmware assurance for one exact, supported lab target**. Do not implement production flashing in this benchmark slice. The initial JSON corpus uses profile ID `synthetic-uefi-uki-v1` and is deliberately synthetic: it exercises UEFI Secure Boot/UKI trust predicates together with generic update-metadata threat cases. It names no physical device and does not qualify an OEM BIOS, embedded-controller, router, or switch firmware updater. That requires a later exact-model/revision lab profile.

- **Symthaea:** proposes diagnoses, candidate Rust patches, test cases, and remediation plans. Its model-generated conclusions are proposals, never independent acceptance evidence or execution authority.
- **Nixward:** owns local NixOS/system-transaction policy, exact target and pre-state binding, authorization, execution journal, post-state verification, and recovery disposition.
- **Xenia adapters:** transport typed observations and invoke only operations documented and allow-listed for the exact product/model/revision/software profile. Network reachability does not grant authority.
- **Luminous Platform:** owns fleet workflow, versioned capability/maturity registry, rollout state, evidence retention policy, and independent cross-artifact verification.
- **Mycelix, where integrated:** may carry signed provenance, witness and evidence receipts; it is not the sole identity provider, firmware verifier, local enforcement plane, or availability dependency.

No production mutation, OEM access-control bypass, signature bypass, arbitrary reflash, or blanket vendor-family capability is implied. Alternative operating-system flashing remains a separate, exact-model/revision qualification track.

## 3. Claim vocabulary

Keep these predicates separate; never infer one from another:

1. **Subject identity:** exact vendor, product family, model, hardware revision, component identity where available, and the evidence binding the observation to that subject.
2. **Management scope:** exact endpoint origin/profile, tenant or account binding, adapter identity and source revision, and any channel/jurisdiction dimensions that change support or authority. Management scope is not physical device identity.
3. **Artifact integrity:** digest and byte length of the exact firmware/update artifact; digest algorithm and canonicalization are explicit.
3. **Signature validity:** cryptographic verification of the exact artifact/signature bytes.
4. **Signer authorization:** whether the verified signer is trusted by the applicable, versioned policy for this subject and operation.
5. **Revocation status:** the result of applicable revocation sources, timestamps, and policy. Unsupported or unresolved revocation conditions remain unknown/non-authorizing.
6. **Compatibility:** exact model/revision, component topology, installed software, firmware dependencies, vendor channel/region where applicable, and declared prerequisites match documented constraints.
7. **Freshness and anti-rollback:** metadata version/expiry, local anti-rollback state, clock-quality assumptions, and any relevant monotonic counter are independently checked.
8. **Provenance and composition:** source references and available SBOM/build provenance are bound to the precise artifact. An SBOM's presence does not prove the artifact is complete or benign.
9. **Authorization:** a responsible authority approved the exact operation digest, target identity, pre-state digest, policy/profile revision, actor, expiry, and nonce or equivalent freshness evidence.
10. **Effect outcome:** prepared/applied/failed/indeterminate is determined from durable evidence; lost acknowledgement is not proof of no effect.
11. **Boot and runtime outcome:** selected boot target, boot attempt/result, and post-boot health are distinct observations.
12. **Recovery readiness and outcome:** a documented, tested recovery route exists for the exact target; when exercised, recovery is verified independently.

Each claim uses a bounded state such as `verified`, `refuted`, `unknown`, or `not_applicable` with a reason and scope. A policy decision may be `eligible_for_lab_staging`, `blocked`, `indeterminate`, `recovery_required`, or `quarantined`; do not label an entire device unqualifiedly "secure."

## 4. Minimum evidence record

Every record should be canonical, versioned, schema-validated, and digest-bound. The first schema proposal should include:

- schema and predicate version;
- stable record ID and parent/subject record references;
- exact target identity and management-scope profile ID, revision and digest as separate fields;
- exact firmware artifact digest, size, component/version, source reference, and signer/key identity when observed;
- evidence kind, source/collector identity and source revision, capture method, observation time, clock status and freshness limit;
- raw-evidence digest plus redacted reference to customer-controlled storage (never put secrets, private keys, full customer configuration, or unredacted topology in a public receipt);
- verifier identity/version, policy revision/digest, inputs, output, reason code and declared limitations;
- forward-only lineage edges identifying which evidence and prior claims a verifier consumed;
- explicit supersession/revocation links; old evidence is not silently rewritten;
- per-claim state and decision trace; no caller-supplied `passed=true`, coverage percentage, maturity grade or final disposition is trusted as an input.

A digest proves byte equality, not who produced the bytes or whether the observation is truthful. Authentication, signer policy, source identity, device binding, freshness, and physical-world truth are separate assurance concerns.

## 5. Frozen adversarial corpus

Author expected outcomes independently from the verifier being tested. Each case must specify exact inputs, expected claim states, final policy disposition, prohibited side effects, and evidence to retain. Include positive controls, negative controls and genuinely indeterminate cases.

| Family | Required fixture or mutation | Required behavior |
|---|---|---|
| Positive controls | Valid artifact, expected signer, authorized policy, matching model/revision, fresh metadata | Each relevant predicate passes independently; only the declared policy may authorize the next stage |
| Byte integrity | Flip or truncate artifact bytes; change size or digest algorithm declaration | Reject integrity claim; preserve exact failure reason |
| Signature | Unsigned, malformed, invalid signature, wrong signer, valid but unauthorized signer | Distinguish missing/malformed/invalid from cryptographically valid but unauthorized |
| Revocation | Direct revoked digest/certificate; timestamped revocation with missing, invalid, conflicting or unsupported timestamp evidence | Definite veto where established; unresolved paths remain unknown/non-authorizing |
| Scope | Wrong vendor/model/hardware revision/component, product family, software train, endpoint or regional channel | Reject scope mismatch; no wildcard or brand-level inference |
| Update metadata | Expired/stale metadata, rollback version, freeze, mix-and-match metadata, insufficient threshold signatures, compromised single key within policy model | Reject per pinned update policy; record which check failed |
| Provenance/SBOM | Missing provenance, digest mismatch, artifact/source mismatch, incomplete inventory, stale component identity | Keep provenance claims incomplete; do not infer absence of vulnerabilities from missing records |
| Identity/freshness | Device-path reuse, changed hardware identity, replayed observation, stale approval, untrusted clock, conflicting observations | Reject or remain indeterminate; require fresh, exact binding |
| Endpoint/adapter | Redirect to another origin, TLS/SSH identity mismatch, tenant mismatch, API/version drift, read-only adapter submitting a mutation | Deny before sending credentials or performing side effects; emit redacted evidence |
| Transaction ambiguity | Timeout before dispatch versus timeout after dispatch; crash after apply; lost acknowledgement; duplicate request | Distinguish proven no-effect from indeterminate effect; never blindly replay a possibly non-idempotent action |
| Boot/runtime | Signature passes but wrong image is selected; selected vs attempted vs successful boot mismatch; post-boot health is stale | Keep the predicates separate; do not claim successful deployment |
| Recovery | Missing recovery path, untested fallback, interrupted update, failed rollback, target becomes unreachable | Block qualification or enter recovery/quarantine state; never convert the missing result to success |
| Privacy/availability | Secret injected into source output; evidence store unavailable; Mycelix/network partition; verifier unavailable | No secret leakage; local policy remains enforceable; consequential action remains blocked where required evidence cannot be checked |
| Parser/schema | Unknown schema version, duplicate IDs, dangling/cyclic lineage, oversized input, unsupported critical field or malformed encoding | Bounded parser rejects or returns explicit unknown; no permissive interpretation |

Freeze case IDs and expected dispositions before scoring. The initial synthetic corpus is [`docs/fixtures/firmware-assurance-corpus-v1.json`](fixtures/firmware-assurance-corpus-v1.json), with **44 reviewable cases (9 positive, 26 negative, 9 indeterminate controls)** and a strict Draft 2020-12 schema at [`docs/schemas/firmware-assurance-corpus-v1.schema.json`](schemas/firmware-assurance-corpus-v1.schema.json). It is explicitly synthetic and names no physical target. Every case contains a `required_claims` list that selects which declared claim states participate in the stage decision. The simple policy composer blocks when any required claim is refuted; otherwise it returns indeterminate when any required claim is unknown or not applicable; only all-verified required claims are eligible for lab staging. This makes claim-state composition executable while leaving the prose descriptions as documentation rather than executable firmware attacks. Add deterministic mutations to reach **10,000 evaluated inputs per profile**. The corpus and generated mutations must be reproducible from a versioned seed and digest. Expand based on observed failure modes, not just case count.

## 6. Benchmark protocol and comparative baselines

Run three baselines over the same frozen corpus:

- **B0 — version-only inventory:** reported firmware version plus model-family mapping.
- **B1 — signature-only:** artifact signature verification without full subject compatibility, revocation, freshness, transaction or recovery claims.
- **B2 — evidence-graph verifier:** predicate-level checks, exact scope binding, independent oracle and fail-closed policy composition.

B0 and B1 are deliberately limited comparison baselines, not straw-man implementations. Report exactly which attacks each baseline cannot detect. Do not compare AI confidence scores as if they were ground truth.

The report must contain:

- frozen corpus digest, profile and policy revision;
- exact source commit and verifier binary/build provenance where available;
- toolchain, OS, runner and hardware identity for performance measurements;
- per-case expected and observed predicate states, disposition, and any side effect;
- positive-control false-rejection count, negative-control false-accept count, and unknown-to-pass coercion count;
- replay/rollback/scope-mismatch acceptance counts;
- evidence-lineage completeness and independent-verifier disagreement;
- recovery results against all injected failure modes;
- runtime and peak-memory distributions by phase, with bounded input sizes stated;
- all missing, skipped, queued, timed-out or stale results listed as non-results, never counted as passing.

Correctness and security acceptance take priority over latency. Separate local schema/graph evaluation, cryptographic verification, device I/O and update duration in performance reports. Do not define a latency SLO until the reference machine, bundle sizes, cryptographic suite and operation boundaries are pinned.

## 7. Initial acceptance targets

These are **proposed benchmark thresholds**, not measured results, certifications or universal security guarantees.

### Stage 0 — schema and deterministic semantics

- 100% of required case IDs accounted for, with no silently omitted case;
- 100% agreement with the frozen independent expected decisions;
- zero false accepts across the defined negative controls;
- zero cases where missing, stale, malformed, contradictory or unsupported required evidence becomes `verified`;
- zero side effects in the read-only harness;
- exact byte-level binding of every input fixture and emitted report;
- repeated runs on the same inputs and pinned build produce the same semantic disposition.

### Stage 1 — mutation and independent verification

- At least 10,000 deterministic mutations per profile, with a frozen seed/corpus digest;
- 100% of detected integrity, scope and policy mismatches result in the prescribed denial/unknown state;
- independent verifier implemented separately from the receipt/manifest builder, with adversarial fixtures authored without consulting the implementation's decision output;
- mutation tests cover each evidence field and each lineage edge, including recomputed outer digests that attempt to conceal inconsistent inner claims;
- no claim of independence if the verifier reuses the producer's decision function or trusts the producer's final status.

### Stage 2 — one exact lab target, read-only

- authoritative model/revision and management endpoint are documented and frozen;
- at least two observations are bound to the same exact target, with disagreement represented explicitly;
- exact artifact, signer, policy and freshness evidence are inspected where available; unsupported claims remain unknown;
- evidence can be regenerated from the documented procedure and checked offline by the independent verifier;
- no firmware write or production mutation occurs in this stage.

### Stage 3 — sacrificial-hardware update qualification (later, separate gate)

- exact target/revision and update path explicitly allow-listed;
- signed package, compatibility, anti-rollback, pre-state, approval, maintenance policy and recovery preconditions all verified;
- power loss, network interruption, timeout after dispatch, partial update, failed boot, stale evidence and rejected image are injected on dedicated hardware where safe;
- every case ends with verified expected state, independently verified recovery, or explicit quarantine—not an inferred success;
- promotion to wider lab or production scope requires a new authorization and evidence packet.

Passing an earlier stage never implies passing a later one.

## 8. Implementation sequence — work independent of CI queue

1. Review and cross-link the current Symthaea Secure Boot consent/state code, Nixward UKI trust verifier and timestamped-revocation work, and platform issue #23's adapter/maturity registry. Do not fork a second command authority, journal, generic evidence envelope, or verifier if an existing canonical owner already supplies it.
2. Freeze the claim vocabulary and required-claim policy over the 44-case language-neutral JSON corpus.
3. Use the Rust-first, offline corpus checker in `tools/firmware-assurance-corpus-checker/` as Stage 0. It validates schema/bookkeeping and derives stage dispositions over supplied claim-state assignments; it does not verify artifacts or hardware.
4. Use `tools/firmware-assurance-corpus-checker/tests/reference_oracle.py` as a test-only, standard-library implementation that independently derives claim-state dispositions and runs metamorphic policy checks; keep Python out of the production path.
5. Run formatting, compilation, unit tests and both oracle paths against the exact source revision when a Rust toolchain is available. Preserve logs, tool versions and exact corpus/schema digests.
6. Select one exact, authorized lab target for read-only observations. Do not infer support from a vendor brand, mock, API success or an emulator.
7. Add firmware execution only by satisfying the separate Stage 3 gate and existing Nixward transaction/recovery contracts.

A queued hosted workflow is neither success nor failure evidence. Local deterministic work can proceed while CI is queued, but no test should be described as passed unless it actually executed against the exact reported source.

## 9. Stage 0 corpus checker increment

A Rust-first corpus checker now lives under `tools/firmware-assurance-corpus-checker/` on the associated research branch. It is deliberately narrower than the Stage 1 firmware verifier.

The checker:
- accepts explicit corpus and schema paths and bounds each input at 4 MiB;
- uses a pinned JSON Schema validator dependency and then enforces cross-field contract rules;
- enforces sequential case IDs, exact case counts/profile binding, unique required-claim references, claim-state/disposition consistency, and mandatory read-only assertions;
- derives a stage disposition over synthetic claim states: any required refutation blocks; otherwise unresolved required evidence remains indeterminate; only all-verified required claims are eligible for synthetic lab staging;
- emits a deterministic report containing the exact input byte lengths and SHA-256 digests, schema/contract results, case distribution, and an explicit claim ceiling;
- evaluates claim-state policy combinations but reports zero physical device operations by construction.

The repository workflow is defined in `.github/workflows/firmware-assurance-corpus-checker.yml`; it runs formatting, Rust unit tests, a positive corpus check, a deliberate case-count mutation, the separate Python reference oracle with deterministic mutation/metamorphic checks, and Clippy. The Python file is test-only; the production checker and intended firmware implementation are Rust. The reference oracle is designed to evaluate 10,000 synthetic claim-state assignments, compare two independently expressed policy functions, and assert that changing a non-required claim cannot change the current stage decision. A workflow definition is not itself evidence that any step ran.

The authoring environment for this increment has no `rustc`, `cargo`, or `rustfmt`. Therefore the Rust crate and authored tests have **not** been compiled, formatted or executed locally; the checked-in Python oracle has not been executed directly either. Hosted status for the latest branch head must be checked before interpreting any CI result. A connector-backed structural audit of the checked-in JSON fixture/schema found 44 sequential unique IDs, 20 claim-state fields per case, 795 required-claim references, matching schema/case property inventories, consistent case-kind/disposition mapping, all mandatory read-only assertions, and no structural invariant errors. A separate tool-mode exploratory sweep ran 10,000 synthetic assignments against two separately expressed policy functions: zero baseline mismatches, zero policy disagreements, 7,331 distinct assignments, and output counts of 3,333 blocked, 3,333 indeterminate, and 3,334 eligible-for-lab-staging. This exploratory sweep is not execution of the checked-in Rust/Python files, not JSON Schema meta-validation, and not CI evidence.

Remaining Stage 0 gates: run formatting, compilation, Rust tests, Clippy and the checked-in Python oracle on the exact reported head; confirm schema meta-validation executes successfully; commit a reproducible dependency lock once Cargo resolution can execute; review whether the reference oracle matches the intended stage policy; and preserve exact-head logs and digests. Passing Stage 0 means the corpus contract and synthetic claim-policy compositions are internally consistent—not that the described attacks execute or firmware is verified.

## 10. Standards to reuse

- [NIST SP 800-193 — Platform Firmware Resiliency Guidelines](https://csrc.nist.gov/pubs/sp/800/193/final): protect firmware from unauthorized change, detect compromise, and recover.
- [NIST IR 8259 Rev. 1 (April 2026) — Foundational Cybersecurity Activities for IoT Product Manufacturers](https://csrc.nist.gov/pubs/ir/8259/r1/final): lifecycle security activities and manufacturer/customer information.
- [UEFI Specification 2.11 — Secure Boot](https://uefi.org/specs/UEFI/2.11/32_Secure_Boot_and_Driver_Signing.html): trust databases, revocation and timestamp semantics.
- [The Update Framework (TUF), specification v1.0.36](https://theupdateframework.github.io/specification/v1.0.36/): signed metadata roles, expiry, rollback, freeze, mix-and-match and key-compromise protections.
- [SLSA provenance v1.2](https://slsa.dev/spec/v1.2/provenance) and the [in-toto Attestation Framework](https://github.com/in-toto/attestation): artifact subject binding and production/verification claims.

Adopting a standard's vocabulary is not a claim of formal conformance. Record the exact version and tested subset. Vendor documentation and the exact hardware profile remain authoritative for supported operations and compatibility.

## 11. Claim ceiling

This benchmark can establish deterministic behavior over a defined corpus and bounded lab profile. It cannot establish that every firmware image is safe, that a signed image is benign, that every OEM implementation follows its documentation, that hardware measurements are truthful, that every fleet device is covered, or that a target is certified or production-ready.

A valid result should be comfortable saying **verified within this scope**, **refuted**, or **unknown**—and should clearly state what evidence would resolve the unknown.
