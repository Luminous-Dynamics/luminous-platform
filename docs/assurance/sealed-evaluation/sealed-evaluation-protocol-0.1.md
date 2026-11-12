# Sealed assurance evaluation protocol 0.1

**Status:** initial executable verification slice; synthetic-only; not a certification scheme  
**Parent:** [OMSI-EVAL-001 / Platform #325](https://github.com/Luminous-Dynamics/luminous-platform/issues/325)  
**Related execution-boundary owner:** [Platform #195](https://github.com/Luminous-Dynamics/luminous-platform/issues/195)  
**PNT measurement pilot:** [Platform #335](https://github.com/Luminous-Dynamics/luminous-platform/issues/335)  
**Implementation issue:** [Platform #340](https://github.com/Luminous-Dynamics/luminous-platform/issues/340)

## Research thesis

A public test set proves that an evaluator and implementation agree on known cases. It does not establish whether a system can discover hidden failure structures. Blind evaluation must separate the SUT-visible task bundle from evaluator-only truth, freeze the latter before execution, and verify the commitment after candidate outputs have been locked.

NIST announced its Artificial Intelligence Technology Evaluation (AITE) programme on 27 July 2026, describing a sequestered testbed with blind data to mitigate train/test contamination. NIST's 2026 TEVV-Athlon initial public draft describes a customizable four-stage assessment process. These are external design comparators; this protocol is a narrower byte-commitment and evidence-ordering slice.

## Roles and trust boundaries

- **SUT:** receives only public task inputs and permitted evidence.
- **Implementation operator:** builds/configures the SUT; must not control private truth custody.
- **Evaluator custodian:** retains private challenge truth, hidden dependency edges, expected outcomes and secret salt.
- **Runtime monitor:** emits access/coverage events. Reuse the independent monitor and coverage receipts in Platform #195.
- **Offline verifier:** recomputes commitment/reveal consistency from exact bytes and evaluates the supplied order/access receipts.
- **External measurement authority:** provides traceable physical reference measurements where a field claim requires them.

Initial synthetic testing may still have shared organizational custody. Report that limitation explicitly. A second programming language does not automatically establish independent custody or organizational independence.

## Protocol stages

1. **Freeze public contract.** Pin run/suite/profile identity, exact public input bytes, decision policy digest, evaluator build digest and allowed resource classes.
2. **Commit private truth.** Generate a 32-byte secret salt. Compute a domain-separated SHA-256 commitment over length-prefixed fields and exact private-bundle bytes. Publish the commitment receipt before SUT execution; retain salt and private bytes with the evaluator custodian.
3. **Execute under monitor.** Run the candidate against only the public bundle. The runtime monitor records attempted and successful private-oracle/grader access, sequence/cursor continuity, resource access and source receipt.
4. **Lock outputs.** Freeze candidate output bytes and record their exact digest before the evaluator reveals private truth.
5. **Reveal and reconstruct.** After output lock, reveal the salt and exact private bytes. The offline verifier checks the commitment, identities, digests, byte length, ordering assertions and monitor coverage.
6. **Score separately.** The evaluator computes outcome metrics from private truth and frozen policy. The scorecard is composed with #337 and the existing #195 signed result/evidence path; this crate does not invent a signer or trust root.

## Commitment construction

The 0.1 algorithm uses SHA-256 with the fixed domain separator:

`LUMINOUS-SEALED-ASSURANCE-COMMITMENT\0v0.1\0`

Then, in this exact order, it appends each field as an unsigned 64-bit big-endian byte length followed by field bytes:

1. Commitment schema identifier.
2. Run ID.
3. Suite ID.
4. Protocol version.
5. Public input manifest SHA-256 (lowercase hexadecimal bytes).
6. Scoring policy SHA-256.
7. Evaluator build SHA-256.
8. Commitment sequence encoded as unsigned 64-bit big-endian bytes.
9. Custody receipt SHA-256 string (empty only in public synthetic fixtures; a real blind evaluation requires a custody receipt).
10. Exactly 32 secret salt bytes.
11. Exact private bundle bytes, without parsing or re-serialization.

The verifier checks both the revealed private bundle SHA-256 and the commitment digest. Do not use platform-native integer encoding or JSON reserialization for this calculation.

The salt must be generated with a cryptographically secure random source outside this crate. This crate validates length and commitment reconstruction; it does not generate or safeguard secret entropy.

## Ordering and access semantics

The supplied event sequence must satisfy:

`commitment < first_execution ≤ last_execution < output_lock < reveal`

The order receipt must bind the same run ID, commitment sequence, candidate output digest, monitor receipt and custody receipt.

- Any attempted private-bundle or grader access is a rejection, even if blocked.
- Any successful access is a critical rejection.
- Missing monitor source, receipt, custody receipt, coverage, sequence continuity, or zero-drop guarantee is INDETERMINATE.
- An event-sequence receipt is only as trustworthy as its issuing monitor/custody source. This library checks shapes, digests and internal consistency; it does not authenticate receipt signatures or prove complete OS/network mediation.
- A commit/reveal verification pass proves exact-byte consistency, not that the evaluator kept the secret or that the hidden truth is physically correct.

## Public versus private manifest

The SUT-visible `PublicRunManifest` has a closed field set. Unknown fields such as `private_bundle_sha256`, `expected_outcomes`, `salt_hex`, or `hidden_dependency_edges` fail parsing. Private truth is carried only in the evaluator custody path and is not committed into this public repository.

The synthetic test fixtures are intentionally public and therefore do **not** demonstrate blind evaluation. They test the verifier mechanics and negative controls only.

## Initial challenge scenarios

The first hidden dependency evaluation should include:
- Different direct providers with one hidden common upstream clock/oscillator.
- Shared power with distinct clock lineage (type separation).
- Hidden common correction provider behind provider aliases.
- Event-time-valid dependency that is expired at reveal/current time.
- Stale or revoked dependency evidence behind a valid signature.
- Inventory omission, snapshot-coverage gap and delayed source frontier.
- Complete positive-control inventory with no declared common ancestor.
- Candidate attempts to inspect oracle/grader artifacts; blocked and successful access are separate outcomes.
- Monitor loss or sequence-gap injection; clean-looking SUT output must remain INDETERMINATE.

The public Mycelix 12-case challenge suite remains the public conformance layer. The blind suite must be held by a separately controlled evaluator custodian and must not be copied into the public repository.

## Measures

Internal proposed targets, not achieved outcomes:
- 100% valid commitment/reveal reconstruction.
- Zero acceptance of altered bytes, salt, run/suite, policy, evaluator, candidate output or sequence ordering.
- Zero accepted private-oracle/grader access attempts.
- 100% INDETERMINATE on monitor loss, missing custody, receipt discontinuity or incomplete access coverage.
- 100% accounting of planned, started, completed, failed, skipped and invalidated attempts, composed from #195.
- Report P50/P95/P99 verification latency and peak memory by private-bundle size.

Do not combine these with physical time error, measurement qualification or decision regret. Each is a separate score family.

## Claim ceiling

The 0.1 implementation verifies exact-byte commitment/reveal consistency and internal sequence/access assertions. It does not authenticate monitor/custody receipts, prove evaluator secrecy, prove runtime isolation or establish physical measurement truth. Stronger claims require the authenticated #195 execution-boundary evidence, separately controlled custody and appropriate external measurement authority.
