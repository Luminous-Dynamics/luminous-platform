# OpsBench Operational Assurance Case v0.1

**Status:** structured assurance-case prototype with a synthetic example. It is not a cryptographic verifier, execution authorization, production safety certification or proof of production readiness.

## 1. Why a case, not a score

Operational agent systems combine different kinds of evidence:
- authority says whether an action was permitted;
- causal evidence says which attempts and external effects occurred;
- publisher verification says who authenticated an artifact;
- a business oracle says whether the requested outcome occurred;
- an SLO observer says whether the service remained healthy;
- a safety evaluator says whether prohibited side effects occurred;
- benchmark controls say whether the trial was valid;
- accounting says what the verified outcome cost.

No one of these subsumes the others. A correct task outcome cannot compensate for an unauthorized effect. A signed event does not prove that its claim is true. A healthy endpoint does not prove business correctness. Low cost does not erase missing observer coverage.

NIST describes an assurance case as a reasoned, auditable artifact that links claims to structured arguments and evidence, with assumptions made explicit. This prototype applies that idea per operational episode.

References:
- NIST SP 800-160 Vol. 2 Rev. 1, [Developing Cyber Resilient Systems](https://csrc.nist.gov/pubs/sp/800/160/v2/r1/final) — system security engineering and assurance-case concepts.
- NIST SP 800-160 Vol. 1 Rev. 1, [Systems Security Engineering](https://csrc.nist.gov/pubs/sp/800/160/v1/r1/final) — evidence-backed system security engineering across the lifecycle.

## 2. Top-level claim

The root claim is:

> Episode E is eligible for human review for the exact declared operational scope and observation horizon.

The claim is deliberately not 'the agent is safe', 'the system is production-ready', or 'the next action is authorized'. It is a scoped, evidence-backed review recommendation.

Required subclaims are selected per scenario:
1. **Authority:** the exact grant/policy allowed this attempt, operation, target and tenant.
2. **Causal integrity:** participant admission/effects, attempt identity and recovery trace are consistent.
3. **Publisher authenticity:** required producer artifacts passed their independent signature/identity/transparency verifier.
4. **Objective outcome:** the independent business oracle established the intended postcondition.
5. **Service SLO:** the independent observer established sustained recovery where required.
6. **Safety:** the evaluator completed the declared audit scope and found no critical findings.
7. **Study integrity:** the corpus commitment, split controls and access audit are valid for benchmark trials.
8. **Economics:** fully loaded cost and human effort are reconciled when cost-effectiveness is claimed.

Economics may be optional for a safety/review decision, but an unsupported economics claim cannot be displayed as verified.

## 3. Graph structure

The input contains five node classes:
- **Claims:** stable ID, claim type, text, required/optional flag and linked argument.
- **Arguments:** target claim, inference rule, premise claim IDs, evidence IDs, assumption IDs and defeater IDs.
- **Evidence:** exact artifact digest, evidence type, upstream verifier result, source/verifier identities and trust domains, source commit, run/episode/arm/scope binding, optional attempt binding, observation time and expiry.
- **Assumptions:** explicit statement, owner, scope, rationale, review time, expiry and accepted/rejected/unknown status.
- **Defeaters:** contradictory facts, missing conditions or alternate explanations that may undermine a claim; each has criticality, status and evidence for mitigation/disproof.

The episode case declares a set of attempt IDs. A case can cover a multi-step Saga or several participant attempts. Each attempt-scoped evidence record must name one of those declared attempts; run-level evidence may use a null attempt ID. A focal attempt ID, when present, must belong to that set.

The graph evaluator rejects duplicate IDs, dangling links, orphan evidence/assumptions/defeaters, missing required claim types and cyclic premise graphs. A root argument must include every mandatory subclaim.

## 4. Deterministic claim propagation

Each claim is derived from the evidence and argument nodes connected to it:
- Any refuted evidence, rejected assumption or unresolved critical defeater refutes the claim.
- Missing required evidence, indeterminate verifier output, unknown/expired assumption or unresolved non-critical defeater makes the claim indeterminate.
- A claim is supported only when all required evidence and premises are supported and all assumptions/defeaters are resolved.

Top-level dispositions:
- `blocked`: a mandatory claim is refuted or a critical defeater remains open or lacks supported mitigation evidence.
- `indeterminate`: a mandatory claim is missing/unknown/expired or a required defeater is unresolved.
- `eligible_for_human_review`: every mandatory claim is supported and no unresolved defeater remains.

An `example_only` input always yields an `example_only` report status, even when its synthetic graph is internally complete. The report carries the hypothetical computed disposition separately. The eligibility boolean is false for example data. `automatic_execution_authorized` is always false.

## 5. Evidence boundaries

The evaluator checks structural binding and consumes the verdicts of upstream verifiers. It does not:
- verify DSSE/Cosign/SPIFFE signatures or transparency proofs;
- authenticate evidence publishers;
- re-run the causal trace verifier, business oracle, safety assessor or SLO evaluator;
- prove observer independence in deployed infrastructure;
- mint authority or call an execution connector.

The report explicitly says publisher authenticity was not cryptographically verified by this aggregator. This does not negate a properly verified upstream artifact; it prevents the aggregator from claiming to have performed that verification itself.

Evidence must bind the exact run, episode, arm, scope and, when present, declared attempt. Each verifier must run outside the executor trust domain and outside the source's own trust domain. These are structural checks; actual identity authentication remains the job of the upstream publisher verifier.

## 6. Run

The standard-library evaluator consumes one case file:

    python3 tools/opsbench-assurance-case-v0.1/evaluate_assurance_case.py \
      --case benchmarks/opsbench-assurance-case-v0.1/assurance-case.example.json

The exact-head CI workflow validates the input and generated report against Draft 2020-12 schemas, runs the evaluator and tests graph, evidence, assumption, defeater and non-compensatory behavior.

## 7. How it composes with the other work

| Evidence layer | Producer/validator work |
|---|---|
| Authority and autonomy boundary | ITOPS/autonomy boundary #167 and authority adapter |
| Attempt/effect and causal recovery | OpsBench/Saga, Nixward journal and Platform causal verifier #174/#177 |
| Publisher authentication | in-toto/DSSE/Sigstore/SPIFFE profile #178 |
| Customer outcome | evaluator-owned business oracle |
| Sustained service health | SLO recovery evaluator #181 |
| Orthogonal safety findings | safety sidecar #172 |
| Confirmatory corpus integrity | sealed corpus and access controls #180 |
| Comparative uncertainty and economics | paired assurance statistics #179 and cost sidecar #170 |

The assurance case is an aggregation/reporting layer. It is not a replacement for any of these independent validators and does not introduce a second transaction journal, authority service or lock. Nixward remains the local host-transaction authority.

## 8. Promotion gates

Before a real episode can be marked eligible for human review:
1. Every mandatory evidence type is present and its upstream verifier result is bound to the exact run/episode/arm/scope.
2. Evidence publisher authenticity is independently verified and the verification receipt is itself provenance-bound.
3. Required assumptions are accepted, in-scope, reviewed and not expired.
4. No critical or unresolved defeater remains, including critical defeaters attached to optional claims.
5. Independent business outcome, SLO and safety claims are supported where required.
6. The root argument's premises include all mandatory claims; no status is inferred from a composite score.
7. Human review remains an explicit decision boundary. This report never authorizes the next mutation.

The synthetic fixture is designed to produce an internally complete hypothetical argument. It is not evidence of real Luminous service performance or system safety.