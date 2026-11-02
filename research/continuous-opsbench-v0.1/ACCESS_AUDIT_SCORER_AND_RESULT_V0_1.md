# Runtime access audit, scorer validation and signed result receipt v0.1

## Purpose

This is a structural evidence layer for issue #195. It closes the next gap after the sealed run freeze (#193) and independent runtime measurement (#196): a benchmark must know what the evaluated system attempted to read, what the runtime allowed/denied, where its monitor was blind, and whether the scorer produced correct known-answer outcomes.

The access audit and scorer are separate evidence sources. The evaluator does not accept agent self-report as access truth or grading truth.

## Runtime access audit

### Required coverage

Each planned (trial, Candidate/Baseline) arm must have exactly one AccessAuditCoverage receipt from the already-frozen runtime observer identity. It binds:
- run, trial, arm and manifest commitment;
- observer source, control domain and adapter digest;
- runtime session identity and event-stream commitment;
- state-isolation, artifact-access and network-egress policy digests;
- measurement interval and clock uncertainty;
- first/last sequence, event count, heartbeat count, dropped count and gap count;
- complete/partial coverage status.

The coverage interval, after clock uncertainty is considered, must enclose the independently validated runtime measurement interval from #196. Each access event binds to the same run/trial/arm, manifest, observer session, source and control domain. Per-arm access sequences must start at one, be contiguous, unique and monotonic within clock uncertainty. Missing coverage, a monitor restart without a continuity transition, dropped events, sequence gaps or unplanned events cannot be interpreted as no access.

### Access events

Every event captures operation (artifact/workspace/cache read/write, DNS, connect, request, response), resource class, keyed pseudonymous resource token, decision (allowed/denied), bytes requested/returned, and content/destination/allowlist/state-namespace digests where relevant.

The evaluator never needs raw filesystem paths, credentials, private oracle bytes or raw request identifiers. The monitor should emit keyed pseudonyms and digests; the key stays outside the evaluator.

### Policy semantics

- Case payload, declared public evidence, toolchain and the current trial's workspace/cache namespace may be permitted by the frozen contract.
- Any attempted PrivateOracle, Grader, Secret or Unknown resource-class access is a policy violation, even if denied. An allowed access is counted as a successful leak.
- DNS, connection, request and response operations must include destination identity and an exact nonzero allowlist-entry digest. A request without that proof is disallowed egress. Denied attempts remain visible; allowed requests without a valid allowlist entry are disallowed successes.
- Workspace/cache reads or writes require the exact writable namespace from the corresponding runtime attestation. A different namespace is a cross-trial state event and makes the run ineligible.
- A read-only common artifact is not a writable shared state namespace. The monitor must classify its resource correctly and bind it to the frozen artifact policy.
- Unknown resource class is a violation, not a permissive default.
- Access counts are attempts and decisions, not merely returned bytes. A denied zero-byte secret/oracle access is still an attempted violation.

### Access report

The validator reports per-arm and run totals for:
- artifact read/write attempts;
- oracle attempts and successful accesses;
- grader and secret attempts;
- unauthorized artifact attempts/successes;
- network events, allowlisted events, disallowed egress attempts/successes;
- workspace/cache events and cross-trial state events;
- monitor coverage and total policy violations.

Certification eligibility is derived by the evaluator: coverage must be complete and no policy violation may be present. The summary is not a signature and is not independent of its input source. Typed source-evidence adapters must authenticate it before final certification.

## Scorer known-answer validation

The scorer is frozen by identifier, version, evaluator digest and scorer digest. It is tested on a sealed known-answer control set whose expected labels come from evaluator-owned truth or blinded adjudication, never the agent or scorer itself.

Each control binds run/manifest, control ID, category, expected label, actual scorer decision, output digest, scorer digest and independent source/control domain. Each category has minimum positive/negative counts and sensitivity/specificity thresholds. Duplicate or missing controls, scorer digest drift, insufficient positive/negative support, source-domain collision and invalid output digests are rejected. A scorer below its preregistered threshold may produce a diagnostic report but cannot authorize certification.

Sensitivity and specificity are reported with counts:
- TP / (TP + FN)
- TN / (TN + FP)

A category with zero positive or zero negative examples is indeterminate, never 100%. This structural validator does not prove semantic truth of labels; ambiguous categories still require blinded independent human adjudication and inter-rater agreement controls. The grader is a measured component, not the oracle.

## Final result receipt

A result receipt is a typed composition of already validated evidence digests:
- manifest commitment and freeze receipt;
- candidate/baseline identity digests and evaluator digest;
- scenario/reveal set digest;
- sealed run ledger digest and full planned/started/terminal denominator;
- runtime attestation summary digest;
- access audit summary digest;
- scorer validation summary digest;
- optional exact request-impact summary digest (#192);
- separately reported safety, outcome, completeness, cost and cluster-statistic digests.

The structural validator recomputes the eligibility decision from supplied summaries. It rejects a caller-supplied certified=true field. It does not sign the result or compute canonical evidence hashes. The existing typed source-evidence authority (#190/#191) must authenticate and sign the exact canonical receipt bytes, bind the intended recipient and run subject, enforce freshness and durable anti-replay. No new trust root, signature scheme or journal is introduced.

## Claim ceiling

This module validates access records and scorer/result composition. It does not prove:
- the OS, hypervisor, network stack or device mediated every access;
- the access monitor could not be detached, bypassed, compromised or made to suppress events;
- an allowlist destination is safe merely because it has a digest;
- a semantic scorer's known labels are correct when they require human judgment;
- a signed result is trustworthy until the existing source-evidence verifier authenticates it;
- the candidate is better than SOTA.

Those claims require the independent runtime/hardware attestation track (#196/#197), source authentication/replay (#190/#191), blinded scorer validation, and matched sealed experiments. A monitor gap is unknown, never evidence of no leakage.

## Boundaries

Platform Saga coordinates episodes; the broker controls mutation authority; participant-owned ledgers own external effects; independent service/business observers own outcomes; Nixward remains the only host transaction/interlock/journal/recovery authority. This evaluator creates no new journal, policy engine, participant ledger or workflow engine.
